# Part 8: Pipelines and expressions

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 56 to
  59 and 47 and 48 of the Decision section
- **Depends on**: [receiver modules](04-receiver-modules.md) for the receiver
  call
- **Needed by**: nothing else in this proposal

## Scope

The separation between local expression evaluation, which `.` performs, and
planner-visible dataflow, which `|` constructs, with one vocabulary for the
pipeline side.

## Rules

### Two different operators

```hql
items
| map(x => x.trim().lower())
```

has one pipeline step, `map`. `trim` and `lower` are expression calls inside
the lambda, evaluated locally, invisible to the planner. `|` is never an
overloaded operator and `.` never creates a step.

### Vocabulary

| Term | Meaning |
| --- | --- |
| pipeline | the logical chain connected by `\|` |
| step | one planner-visible operation occurrence in a pipeline |
| operation | the semantic behavior a step invokes |
| stage | a physical execution unit the planner creates, holding one or more steps |
| backend | what executes stages |

A step is not a stage. A stage is never source syntax.

## Current state

Already decided and implemented:

- `x | f(a)` is typed as `f(x, a)` and lowers to a step node the executor sees
  ([HQL-0004](../../HQL-0004/README.md)); a step is not a stage
  ([HQL-0005](../../HQL-0005/README.md)); the backend is host configuration,
  with `localhost` as the oracle.
- Every step declares purity and a shape (`Elementwise`, `Partition`,
  `Reduction`, `Barrier`), which is what a planner may assume.

Not present:

- A call on a value. `.` is field access; a function value in a field is not
  callable by `.` today.

## What changes

- Nothing on the pipeline side: the draft restates HQL-0004 and HQL-0005. The
  vocabulary table above is the one those records use, and the
  [pipes card](../../../wiki/hql/types/pipes.hmd) already distinguishes a
  function from a step.
- The expression side gains the receiver call, which is specified in
  [part 4](04-receiver-modules.md). This part only fixes that the receiver call
  is not a step and that an operation reachable both ways,
  `xs | sort` and `xs.sort()`, is one declaration with one signature; the
  pipe form is a step and the dot form is not. Whether a planner may still see
  through a dot-call to a pure core operation is an optimization, not
  semantics.

## Work units

1. One sentence in the pipes card and in HQL-0005: the receiver call is
   expression evaluation inside a step.
2. A test that `xs | map(x => x.trim())` lowers to one step.
