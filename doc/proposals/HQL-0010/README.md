# HQL-0010: `tee` branches a pipeline without ending it

**Status**: drafted
**Created**: 2026-10-18
**Source**: [HQL pipes](../../wiki/hql/types/pipes.hmd)

## Abstract

`tee` is a pipeline stage. `input | tee x = branch | next` evaluates `branch`
on the current pipeline value, binds the result to `x`, and passes the input
value to `next` unchanged. Its type is `T -> T`; the branch has its own type
`T -> R`, and `R` never affects the type that flows through. `tee` fixes query
semantics only: whether the shared input is streamed to both consumers,
buffered, materialized, cached or replayed is an executor choice, and every
choice MUST be observationally equivalent to both computations receiving the
same logical value. `tee` fans out and never fans in; recombining a branch with
the main pipeline stays an explicit operation.

## Motivation

- An intermediate pipeline value is often needed more than once: take the first
  three filtered cards as a sample, count them, and keep traversing from the
  full filtered set.
- Variables already express this, but they split one logical query into
  several statements and do not say that a computation is a side branch of one
  particular stage:

  ```hql
  filtered = cards | filter(_.year > 2020)
  first    = filtered | take(3)
  result   = filtered | related(Mentions)
  ```

- A pipeline may be a finite collection, a lazy query, a
  [`Stream<T>`](../HQL-0006/README.md), an event sequence or a long-running
  computation. Spelling the branch with variables reads as though `filtered`
  is a stored value, which suggests a materialization the query does not need.

## Goals

- a stage that names a side computation at the point in the pipeline it
  derives from
- a contract independent of execution strategy, so that a backend keeps the
  freedom described in [HQL-0004](../HQL-0004/README.md)

## Non-goals

- replacing assignment; variables remain the general way to name independent
  expressions
- a general `fork` that ends the pipeline in several first-class results
- any implicit recombination (join, zip, union) of a branch with the main
  pipeline

## Specification

### 1. Name

The stage is named `tee`, after the Unix `tee` command and the plumbing
T-junction that command is named for: one incoming flow is sent into a side
branch while the original flow continues.

```text
                 ┌── side query
                 │
input ── tee ────┼──────── original input continues
```

### 2. Syntax

The single-expression form binds a name to a branch:

```hql
cards
| filter(_.active)
| tee sample = take(3)
| related(Mentions)
```

- A branch is a pipeline whose first stage receives the `tee` input.
- A multi-stage branch MAY be written over several lines after `=`:

  ```hql
  cards
  | tee sample =
      take(3)
      | map({title: _.title})
  | related(Mentions)
  ```

- A block form MAY be added for long branches; its exact syntax is decided
  separately and does not change the contract below.

  ```hql
  cards
  | tee stats {
      group(_.author)
      | size
  }
  | related(Mentions)
  ```

### 3. Semantics

For

```hql
input
| tee x = branch
| next
```

the program MUST behave as though both computations receive the same logical
`input`:

```text
x      = branch(input)
output = next(input)
```

- `branch(input)` MUST NOT change the value `next` receives.
- The type flowing through `tee` is the type of its input:

  ```text
  tee    : T -> T
  branch : T -> R
  ```

- `R` is independent of `T`. In

  ```hql
  cards
  | tee total = size
  | related(Mentions)
  ```

  the main pipeline is `Stream<Card>` and `total` is `Int`.

The example in the Abstract is therefore equivalent to:

```hql
filtered = cards | filter(_.year > 2020)
first    = filtered | take(3)
result   = filtered | related(Mentions)
```

### 4. Execution

`tee` defines what a query means, not how it runs. A backend MAY:

- consume both branches concurrently;
- buffer values;
- materialize the shared input;
- cache the common source;
- replay a reproducible source;
- fuse the branch into the surrounding plan.

Every such strategy MUST be observationally equivalent to both branches
receiving the same logical value. The syntax MUST NOT imply a materialization
the semantics do not require, because the input may be unbounded.

The planner MUST recognize shared ancestry between a branch and the main
pipeline. This is also what makes common-subquery reuse, shared traversal and
coordinated execution of branches available as later optimizations.

### 5. Relationship to variables

`tee` sits beside assignment, not in place of it.

- Use a variable when independent expressions need names:

  ```hql
  base = cards | filter(...)
  a    = base | ...
  b    = base | ...
  ```

- Use `tee` when the branch belongs to the query and derives from one point in
  it:

  ```hql
  cards
  | filter(...)
  | tee a = ...
  | ...
  ```

### 6. Relationship to `fork`

`tee` is not a general `fork`. A future `fork` would end the current pipeline
and produce several first-class results:

```hql
source
| fork {
    a: ...
    b: ...
}
```

```text
             ┌── A
source ──────┤                 result = {a: A, b: B}
             └── B
```

`tee` keeps one main pipeline running and adds side results:

```text
             ┌── side branch
source ─ tee ┴──────── main pipeline continues
```

The two MUST stay distinct operations.

### 7. No implicit fan-in

`tee` does not define how a branch result is combined with the main pipeline.
A later computation that needs both MUST say so explicitly, through a record
construction, `join`, `union`, `zip`, an aggregation or another composition
operator.

### 8. Worked example

```hql
cards
| filter(_.year >= 2025)
| tee sample =
    take(3)
    | map({title: _.title})
| tee total = size
| related(Mentions)
| group(_.concept)
| size
```

Three computations read the filtered cards:

```text
filtered cards
    ├── take(3) -> map(...)       -> sample
    ├── size                      -> total
    └── related(Mentions)
        -> group(...)
        -> size                   -> main result
```

The main query stays one pipeline while the intermediate observations are
kept.

## Backwards Compatibility

`tee` is a new reserved stage name. No existing program changes meaning; a
program that uses `tee` as an identifier would have to rename it.

## Reference Implementation

- the pipeline grammar, once the bootstrap parser accepts pipelines (see
  [HQL pipes](../../wiki/hql/types/pipes.hmd))
- a `tee` step node carrying the binding name and the branch region
- the checker: the step's output type is its input type, and the binding takes
  the branch's result type
- the `localhost` backend: evaluate the shared input once, then the branch and
  the continuation over it in program order
- [HQL pipes](../../wiki/hql/types/pipes.hmd) gains a section on `tee`

## Test Plan

Unit tests MUST include:

- a `tee` whose branch changes type (`size`) leaves the pipeline type unchanged
- the bound name is visible after the pipeline and holds the branch result
- the main pipeline's result equals the same pipeline with the `tee` stage
  removed

Integration tests MUST include:

- the worked example, with `sample`, `total` and the main result checked
  against the variable-based equivalent

## Open Questions

- Does `tee` require a named binding, or is an unbound side-effecting branch
  allowed?
- What is the syntax for multi-stage branch bodies?
- What are the lifetime and scope of a binding introduced by `tee`?
- How does `tee` behave when the branch is asynchronous or long-running?
- How do failures propagate between the branch and the main pipeline?
- What does `tee` mean over infinite streams and events?
- Should one `tee` expression support several branches?

None of these changes the core contract: `tee` evaluates a branch from the
current pipeline value while passing that value unchanged to the next stage.

## Changelog

- 2026-10-18: drafted
