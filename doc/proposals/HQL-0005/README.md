# HQL-0005: Pipeline vocabulary and the logical/physical separation

**Status**: drafted
**Created**: 2026-09-28
**Source**: [typed pipeline operations](../../issues/0013-typed-pipeline-operations.md)

## Abstract

An HQL pipeline is an ordered sequence of typed steps. A step applies an
operation or a presentation to the value flowing through the pipeline. The
logical pipeline does not prescribe a particular execution strategy. This record
fixes six terms — **pipeline**, **step**, **operation**, **presentation**,
**operator** and **stage** — and assigns each exactly one meaning. The first
five describe the program; **stage** is reserved for the execution planner,
where one or more steps are grouped into a physical unit of execution. An
operation is a value of a reserved function type, so a user-defined function or
lambda of that type is an operation on equal terms with the standard
vocabulary (of hql reserved keywords). A stage is not a node of its own: the planner annotates each step
with the stage it belongs to, and the annotated pipeline is the schedule it
distributes. The knowledge base, the Rust host and the corpus comments adopt the
vocabulary together.

## Motivation

The words have drifted, and the drift is starting to encode a design decision
nobody made.

- [HQL-0004](../HQL-0004/README.md) defines the pipe as a "stage boundary" and
  describes every step as a stage. That makes the logical program and its
  execution plan the same object, which is exactly what a planner must be free
  to change: fusing `filter | map` into one traversal would, under that wording,
  delete a stage from the program.
- [map/reduce for queries](../../models/behavior/map-reduce.md) and the
  [pipes](../../wiki/hql/types/pipes.hmd) card inherit that wording, and the
  `examples/mapreduce/` comments say "three stages" for three steps.
- The Rust registry entry is called `Step` (`src/extensions/mod.rs`), but it is
  the declaration of what a step invokes, not an occurrence in a pipeline. Two
  `filter` steps in one pipeline share one registry entry.
- `map`, `filter` and the like are sometimes called operators, which collides
  with `|`, `+`, `:` and `->`.

The practical consequence is a language model that reads as one MapReduce cycle.
Once pipelines may be long — streaming, batching, aggregating and presenting in
one program — the planner has to be able to cut a single pipeline into several
execution stages with different strategies, and the vocabulary has to leave it
room to.

## Goals

- Every term has one meaning, and the logical terms never name a physical unit.
- A long pipeline can be planned into several stages without the language
  saying anything about how.
- A user can write an operation, named or as a lambda, and a planner treats it
  exactly as it treats `map` or `filter`.
- A plan has one form: the pipeline's own step nodes, each annotated with its
  stage.

## Non-goals

- **No planning algorithm.** This record fixes the unit a planner produces and
  the form its plan takes; how it chooses stage boundaries is left open.
- **No syntax change.** Named functions and lambdas are already written today;
  what changes is that their type can make them operations.

## Specification

### The six terms

The terms are normative across `doc/`, `src/`, `std/` and `examples/`.

```text
Pipeline      ordered logical program: a maximal chain of steps joined by |
Step          one occurrence, at one position, in a pipeline
Operation     the semantics a step invokes: map, filter, collect, emit, group, reduce, ...
Presentation  consumer-facing rendering a step may invoke instead: table, graph, markdown, ...
Operator      syntax: |  +  :  ->  and the rest of the operator table
Stage         physical, planned execution unit; one or more steps
Backend       the mechanism that executes stages
```

```text
Pipeline
    |
    +-- Step -> Operation
    +-- Step -> Operation
    +-- Step -> Presentation

             | planner
             v

Plan
    +-- Stage (one or more steps) -> Backend
```

- A **step** MUST invoke exactly one operation or exactly one presentation. It
  is identified by its position, so `filter` written twice is two steps and one
  operation.
- `map`, `filter` and every other named thing after `|` MUST NOT be called an
  operator. `|` is the operator that forms a pipeline.
- **Stage** MUST NOT be used as a synonym for step. A stage exists only in an
  execution plan; its boundaries are chosen by a planner and are not visible in
  HQL source.
- HQL syntax MUST NOT expose stage boundaries, backends, or any other physical
  choice. A program that could name one would be claiming its answer depends on
  it, and [HQL-0004](../HQL-0004/README.md) already requires that it does not.

### An operation is a value of a reserved function type

Operations are not a closed list. The language reserves a function type for
them, and anything of that type is an operation:

```hql
// Reserved. One per shape; each is a function type from input to output.
Elementwise<I, O>
Partition<I, O>
Reduction<I, O>
Barrier<I, O>

// Their common supertype.
Operation<I, O>
```

- `Operation<I, O>` and its four shape subtypes are reserved names. A program
  MUST NOT declare a type with one of them.
- Each reserved type is a function type `I -> O` that additionally carries the
  [HQL-0004](../HQL-0004/README.md) shape its name states and a purity. A value
  of the type is callable exactly as an `I -> O` function is.
- The standard vocabulary is declared through these types: `filter` and `map`
  are `Elementwise`, `group` is `Partition`, `size` is `Reduction`, `sort` is
  `Barrier`, each as [HQL-0004](../HQL-0004/README.md) tabulates it. They have no status a user operation cannot also have.
- A **user operation** is a named function or a lambda whose type is one of the
  reserved types. It MAY appear as a step wherever a standard operation may,
  under the same typing rule: `x | f(a)` is typed as `f(x, a)`.
- A function whose type is a plain `I -> O` is still callable from a step's
  arguments (`map(b => summarize(b))`) but MUST NOT itself be a step. A planner
  knows nothing about its shape and so could not place it.
- The checker MUST verify a declared shape it can verify: a function declared
  `Elementwise` whose body reads anything but its one element is a type error.
  A shape the checker cannot verify, such as an associative `Reduction`, is the
  author's claim, and a planner relies on it as it relies on a host extension's
  declared shape.
- Purity follows [HQL-0001](../HQL-0001/README.md): a user operation is pure
  only if every call in its body is pure. An impure user operation is a barrier
  to reordering whatever its declared shape.

A user operation therefore enters the plan on the same information as a host
one. The planner reads shape and purity off the step's operation type and never
off its name.

### A stage is an annotation on steps

Before a backend accepts a pipeline, the planner compiles a **schedule**: it
partitions the pipeline's steps into stages and decides how those stages are
distributed as jobs. It records that decision by annotating the steps, not by
building a second tree.

```text
step node                 annotation
events                    stage 0  streaming
| filter(e => e.active)   stage 0  streaming
| collect(100)            stage 1  batch boundary
| map(b => summarize(b))  stage 2  parallel, 8 jobs
| emit                    stage 3  streaming
| filter(r => r.accepted) stage 3  streaming
| table                   stage 4  presentation
```

- A **plan** is the checked pipeline with every step node carrying a stage
  annotation. The steps, their order and their operations are unchanged; only
  the annotations are added.
- Every step MUST carry exactly one stage annotation, and the steps of one stage
  MUST be contiguous in the pipeline.
- A stage annotation names the stage and the execution properties the backend
  needs to dispatch it: its strategy (streaming, batch, parallel, presentation)
  and its job units. Its exact fields are backend metadata and are not fixed
  here.
- A backend MUST refuse a pipeline that is not fully annotated. Annotation is
  the step at which the pipeline is accepted for execution; there is no
  unplanned execution path.
- Annotation is automatic. The planner derives every annotation from the
  operation type of each step (its shape and purity) and from what the target
  backend can execute; nothing in the program or from its author is required.
- The `localhost` backend's plan is the trivial one: the whole pipeline is a
  single stage, evaluated in process one step after another.
- Annotations are not visible in HQL source and MUST NOT change the value the
  pipeline produces.

### Plan IR

*Placeholder; not yet designed.*

The plan IR is the low-level form of an annotated plan, in the spirit of LLVM
IR rather than a bytecode: the checked query itself, with each step carrying
its annotations, stating what is scheduled where. It is the form a schedule is
inspected and debugged in.

The IR is bound to a backend topology. It states the nodes a plan runs on and
the resources each job requires, so that it says up front what running the
query takes: on a Kubernetes cluster, for example, the resource claims that
must be granted before the query can run at all.

### What survives from HQL-0004

The execution contract of [HQL-0004](../HQL-0004/README.md) stays; only its
nouns change.

- HQL-0004's **region** is this record's **pipeline**. The two are the same
  maximal chain of `|`-joined steps; one name is kept.
- The pipe still lowers to a node the executor sees and is never rewritten into
  a call before a backend receives it. That node is a **step node**.
- **Shape** (elementwise, partition, reduction, barrier) and **purity** remain
  properties of an **operation**. They are what a planner reads when it decides
  where one stage ends and the next begins.
- "Adjacent elementwise stages may be fused" becomes "adjacent elementwise
  steps may be fused into one stage".

### A pipeline is not one MapReduce cycle

MapReduce is one strategy a planner may choose for a suitable run of steps; it
is not the model of a pipeline.

```text
logical pipeline                      one possible plan

events                                streaming stage
| filter(e => e.active)               |
| collect(100)                        batch boundary
| map(b => summarize(b))              parallel stage
| emit                                streaming stage
| filter(r => r.accepted)             |
| table                               presentation stage
```

- A single plan MAY contain streaming, batching, partitioned or parallel
  execution, graph or index access, materialization boundaries and
  presentation, each as its own stage.
- Every plan MUST produce the value the `localhost` backend produces, as
  [HQL-0004](../HQL-0004/README.md) requires.

### The Rust host follows the vocabulary

The registry names what it holds.

- The registry entry currently named `Step` SHOULD be renamed `Operation`, and
  `StepSpec` SHOULD become `OperationSpec`. The entry declares an operation; a
  step is the execution-tree node that invokes it.
- Presentations SHOULD be registered separately from operations, as
  [HQL-0008](../HQL-0008/README.md) specifies, so that the host type says which
  one a step invokes.
- `hql builtins` SHOULD list operations and presentations under separate
  headings.
- A host operation and a user operation SHOULD reach the step node through the
  same operation type, so that the planner has one code path for both.
- Stage annotations SHOULD be a field of the step node, empty until the planner
  fills it, rather than a separate plan structure.

## Backwards Compatibility

No program changes meaning. A program that already declares a type named
`Operation`, `Elementwise`, `Partition`, `Reduction` or `Barrier` stops
checking; the corpus has none. The rename touches Rust identifiers that are
`pub(crate)` or are `pub` only within the `hql` crate's extension module; a
public re-export, if any, is kept as a deprecated alias for one release. The
`--format json` diagnostic field `stage` is a separate use of the word (the
processing phase that reported the diagnostic), and renaming it is a wire
change; see Open Questions.

## Reference Implementation

- `src/extensions/mod.rs`, `src/extensions/spec.rs` — `Step` to `Operation`,
  `StepSpec` to `OperationSpec`, and every call site
- `src/builtins.rs` — separate headings for operations and presentations
- `std/core.hql` — declares the reserved operation types
- the checker — accepts a user function of an operation type as a step, rejects
  a plain function type there, and verifies an `Elementwise` body
- the step node — gains a stage annotation; the `localhost` planner puts every
  step in one stage, and the backend refuses an unannotated pipeline
- `doc/wiki/hql/types/pipes.hmd` — states the six terms and the sentence in the
  Abstract; stops calling a step a stage
- `doc/models/behavior/map-reduce.md`, `doc/proposals/HQL-0004/README.md` —
  "stage" becomes "step" wherever it names an occurrence; "region" becomes
  "pipeline"
- `doc/wiki/hql/presentation.hmd` — "a presenter is a terminal stage" becomes
  "a terminal step"
- `examples/mapreduce/queries/*.hql` — comment wording only

## Test Plan

Unit tests MUST include:

- `hql builtins` output lists operations and presentations under distinct
  headings, and every registered entry appears under exactly one
- a named function and a lambda of type `Elementwise<T, U>` each run as a step
  and produce the same value as the equivalent `map`
- a function of plain type `T -> U` used as a step is a type error
- a function declared `Elementwise` whose body reads beyond its element is a
  type error
- declaring a type named `Operation` is a type error
- the `localhost` plan annotates every step with the same single stage, and the
  backend refuses a pipeline with a missing annotation

A documentation check SHOULD be added or run by hand:

```bash
grep -rn -i "stage" doc/wiki doc/models examples std src
```

Every remaining hit either names a planner/execution unit or is one of the
separate uses listed under Open Questions.

## Open Questions

- Does the diagnostic field `stage` in `--format json`
  ([reporting](../../models/behavior/reporting.md)) become `phase`? It names
  parse/check/evaluate, not a planner unit; keeping it leaves a third meaning of
  the word in a public format, and renaming it is a wire change.
- Does vault assembly, which [architecture](../../models/domain/architecture.md)
  describes as coordinated "stages", adopt "phases" as well, so that the
  reservation holds across the whole knowledge base?
- Is "presentation" the name of both the category and the registry entry, or
  does the entry get a separate noun (`Presenter`, as the
  [presentation](../../wiki/hql/presentation.hmd) card currently says)?
- Is purity part of the reserved operation type (`Elementwise<I, O>` versus an
  impure variant), or always inferred from the body?
- Do stream operations from [HQL-0006](../HQL-0006/README.md) and
  [HQL-0007](../HQL-0007/README.md) (`collect`, `emit`) need a shape beyond the
  four, or does `Barrier` cover a batch boundary?
- Which fields of a stage annotation are fixed by the language, and which are
  left to each backend?
## Changelog

- 2026-09-28: drafted
- 2026-10-17: operations as a reserved function type; stages as step
  annotations
