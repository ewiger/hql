# HQL-0056: Logical computation model and backend engines: a `Step` run at once or planned

**Status**: drafted
**Created**: 2026-11-13
**Source**: the maintainer's note on the Learn page
[Writing with `target` and `sink`](../../public/learn/09-io/target.md);
[HQL-0055](../HQL-0055/README.md) (steps, autes, stages, tasks and jobs);
[HQL-0004](../HQL-0004/README.md) (backends held to `localhost`);
[HQL-0005](../HQL-0005/README.md) (logical steps and physical planning)

## Abstract

A checked program is a **logical computation model**: a collection of
**logical computation units** and the data and effect dependencies between
them. The type of a unit is `LogicalComputationUnit`; `Aute` becomes its
subtype and `Step` stays the subtype of `Aute`:

```text
LogicalComputationUnit  >  Aute  >  Step
```

A program that ends in `target`, `sink`, a relational mutation or a log call
ends in a `Step`: its final computation step. A `Step` carries no data
value. HQL has no void type, and `Step` stands in for one. A backend engine
receives the whole model, and what happens to its units depends on the
engine. An **immediate engine**, such as `localhost`, runs each unit when
evaluation reaches it. A **planning engine** turns the model into an
execution plan of stages, tasks and a job (HQL-0055) and schedules that
plan, or delegates it to an external engine.
The program observes the same value, type, order of effects and failures on
either kind.

## Motivation

- The Learn page on `target` described `Step` as "the record that a
  statement ran". That reading only holds on an immediate engine. On a
  planning engine the step is planned first and run later, so the sentence
  misleads readers about any backend other than `localhost`.
- HQL-0055 leaves open whether `: Step` reads as "returns a step, a deferred
  computation" or "ran, and gave nothing". This record decides it: the
  program sees a logical unit, and when it runs is the engine's choice.
- HQL-0055 puts `Step` under `Aute`, a unit of *execution*. A program
  describes computation before any engine executes it, so the logical unit
  needs a name above the execution one.
- The registry in `src/backends/mod.rs` holds only `localhost`. The next
  backend will plan before it runs. The difference between the two kinds of
  engine needs a written rule before that backend lands.

## Goals

- One name, the logical computation model, for what checking produces and
  what every engine receives.
- One type that names a unit of logical computation, above `Aute` and
  `Step`.
- One rule for how an engine realizes a model: run its units at once, or
  plan and schedule them.
- A program's meaning that does not depend on which kind of engine runs it.

## Non-goals

- **No planning engine is added.** `localhost` stays the only registered
  backend.
- **No new syntax.** A program cannot ask for immediate or planned
  execution. HQL-0005's rule holds: syntax MUST NOT expose a physical
  choice.
- **No deferred values.** A program cannot hold an unrun unit and run it
  later. `Step` is not a thunk, a future or a handle.
- **No write outcome.** The count, endpoint and commit of a write stay out
  of scope, as in HQL-0055.

## Specification

### 0. The logical computation model

This section names what checking produces and what an engine receives.

- The **logical computation model** of a program MUST be the collection of
  its logical computation units, together with the data dependencies and the
  ordered effects between them (HQL-0055, "Effect is a property, not a
  level").
- Checking MUST produce the complete model before any engine plans or runs
  any part of it. Today the model is the checked execution tree,
  `crate::execution::Program`, whose step nodes are the units.
- A backend engine MUST receive the model, and nothing earlier, as
  `Backend::run` receives `Program` today. An engine MUST NOT add, drop or
  reorder units in a way the program can observe (see "What the program
  observes" below).
- A unit belongs to exactly one model. The model is not itself a unit: it
  has no type in HQL source, and a program cannot name or hold it.

The shape of the model depends on the backend engine:

```text
Engine     Shape  Where a unit used twice appears
immediate  tree   once per use: each use is evaluated where it is written
planning   DAG    once: every use is an edge to the one shared unit
```

- An immediate engine MAY hold the model as a **tree**. `localhost` does:
  the checked execution tree, walked in program order. A shared input, such
  as the one stream a `fork` or `tee` reads, is a node the walk reaches from
  each branch.
- A planning engine SHOULD hold the model as a **directed acyclic graph**
  (DAG). Units are nodes; data dependencies and effect order are edges. A
  unit read by several branches (HQL-0037's fan-out) is one node with
  several outgoing edges, and a join or merge (fan-in) is one node with
  several incoming edges. The planner cuts stages over this graph, which
  HQL-0055 already describes as "a graph of stages, not a list".
- A model MUST be acyclic in either shape. A cycle would make a unit depend
  on its own result.
- The two shapes MUST describe the same computation: the same units, the
  same data dependencies, the same effect order. Converting a tree to a DAG
  merges repeated uses of a unit only when that unit is pure; an effectful
  unit written twice runs twice and stays two nodes (HQL-0055, "Effect is a
  property, not a level").
- The shape is the engine's choice and MUST NOT be visible to the program.

The model is the boundary between the language and the engines. Naming it
lets each engine be specified by what it does with one model, instead of by
how it walks a syntax tree.

### 1. The type chain

This section adds one builtin type above `Aute`.

- `LogicalComputationUnit` MUST be a builtin, abstract type. No program
  constructs a value of it directly.
- `Aute` MUST be declared a subtype of `LogicalComputationUnit`. `Step : Aute`
  is unchanged.
- A parameter of type `LogicalComputationUnit` MUST accept an `Aute` and a
  `Step`, by HQL-0016's subtyping rule. The reverse MUST be refused.

```text
LogicalComputationUnit   a unit of logical computation in a checked program
Aute : LogicalComputationUnit   such a unit, executed atomically
Step : Aute                     such a unit that gives no data value
```

The logical name sits above the execution name because a program is checked
before any engine executes it. Placing `Aute` on top would make the
execution concept the root of the hierarchy.

### 2. `Step` is the final computation step

This section fixes what a `Step` means to a program.

- `Step` is the type of a unit that gives no data value. It replaces a void
  type, which HQL does not have.
- A program whose last statement is a write, a mutation, a log call, a
  binding or a declaration MUST check to `Step`. That statement is the
  program's final computation step.
- A `Step` result MUST print nothing at the top level, so the output of such
  a program is only what its steps wrote.
- Documentation MUST describe a `Step` as a computation step that gives no
  value. It MUST NOT describe it as "the record that a statement ran" or as
  "a computation waiting to run", because each holds on only one kind of
  engine.
- `Never` stays distinct: a `Step` has one value, and `Never` has none.

### 3. Immediate and planning engines

An engine is classified by what it does with the model once checking is
done.

- An **immediate engine** MUST run each unit when evaluation reaches it, and
  MUST finish it before the next unit starts. `localhost` is the immediate
  engine: `Execution::InProcess` in `src/backends/plan.rs`, one stage, each
  step run to completion.
- A **planning engine** MUST turn the model into an execution plan before
  any unit runs. The plan is made of stages, tasks and one job
  (HQL-0055). The engine MAY schedule the plan itself or hand all or part of
  it to an external engine such as Spark SQL.
- A planning engine MUST refuse a unit it cannot plan before any part of the
  program runs, with the `Phase::Plan` / `Kind::BackendRefusal` diagnostic
  that `Registered::execute` already gives.
- On either kind, a `Step` MUST be complete when the program observes it.
  Locally, a step is complete when it returns. On a planning engine, it is
  complete when the engine confirms its effect, as HQL-0055's section on
  `Aute` and `Step` states.

### 4. What the program observes

The kind of engine is invisible to the program.

- A program MUST produce the same value, type, sequence positions,
  diagnostics and order of effects on a planning engine as on `localhost`
  (HQL-0004, HQL-0044).
- A program MUST NOT be able to tell whether a unit ran immediately or was
  planned, except through timing and through backend reports outside the
  program's value (`hql plan`, logs).
- Which engine runs a program stays host configuration: `--backend`, the
  vault's `[execution] backend`, `HQL_BACKEND`, then `localhost`.

## Backwards Compatibility

- A program that declares its own type `LogicalComputationUnit` stops
  checking, because the name becomes builtin. No program in `examples/` or
  `tests/` does.
- `Step` and `Aute` keep their meaning and their values. No program's value
  or output changes.
- HQL-0055's sentences that read `Step` as "a statement ran" are reworded to
  match the section "`Step` is the final computation step" above.

## Reference Implementation

- `src/types/builtin.rs`: add `LOGICAL_COMPUTATION_UNIT =>
  "LogicalComputationUnit"`, `TypeKind::Abstract`, and set it as `AUTE`'s
  supertype.
- `src/backends/plan.rs`: no change for `localhost`. A planning engine adds
  its own `Execution` variant when it lands.
- `src/backends/mod.rs`: the module comment names the two kinds of engine.
- `doc/public/learn/09-io/target.md`: already describes `Step` as the final
  computation step (the source of this record).
- Learn and reference pages that define `Aute` or `Step`; `hql reference`
  regenerates the marked parts.
- HQL-0055: the wording above, and its open question about reading
  `: Step` marked resolved by this record.

## Test Plan

- `tests/`: an `Aute` and a `Step` are accepted where a
  `LogicalComputationUnit` is expected, and the reverse is refused; a
  program declaring `type LogicalComputationUnit` is refused.
- The tutorial suite (`tests/tutorial.rs`) runs the changed book pages.
- `hql reference --check doc/public/reference` is clean.

```bash
cargo test --locked
cargo run --locked -- reference --check doc/public/reference
```

## Open Questions

- Does the DAG form get one shared representation that every planning
  engine builds from `Program`, or does each engine build its own? HQL-0055
  leaves the intermediate representation open; this record names what it
  holds and that it is a DAG.
- When a tree becomes a DAG, which pure units are merged: only those the
  program binds to one name and reads twice, or also two identical
  expressions written separately (common subexpression elimination)?
- Is a program of several pipelines one model, or one model per pipeline
  joined by effect order?
- Is `LogicalComputationUnit` the name, or a shorter one (`Computation`,
  `Unit` once its refusal period ends)? The name is long for a type a
  program may write in a signature.
- Does `hql run` on a planning engine wait for a bounded job to finish
  before it returns the program's value, or return once the plan is
  accepted? For an unbounded job it cannot wait.
- Does a planning engine plan the whole program before running any of it,
  or may it plan and run one pipeline at a time?
- Do subtypes other than `Aute` sit under `LogicalComputationUnit`: a pure
  computation that gives a value, a whole pipeline, a transaction scope?
- Does `Execution` grow a variant per planning engine, or one `Scheduled`
  variant with the engine as a parameter?

## Changelog

- 2026-11-13: drafted, from the maintainer's correction of the `target`
  Learn page: a checked program is a logical computation model, a collection
  of logical computation units, held as a tree or a DAG depending on the
  engine; `Step` under `Aute` under
  `LogicalComputationUnit`; and an engine that either runs a step at once or
  plans it.
