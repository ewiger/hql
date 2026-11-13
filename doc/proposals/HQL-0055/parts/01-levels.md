# HQL-0055: the four levels

Companion to [HQL-0055](../README.md). It states each level with its owner
and cardinality, places each against the records that already use the word,
and works one pipeline through two plans.

## Logical and physical computation

```hql
source("orders")
| filter(o => o.amount > 100)
| map(o => o.customer_id)
| distinct
| sink("customers")
```

The pipeline has a head, `source("orders")`, and four steps, which is how
`hql plan` lists it. How many stages, tasks and jobs it becomes is not in the
program; it is in the plan a backend's planner draws for it.

```text
HQL source program
      |
      v
typed expressions                      the checker (HQL-0044)
      |
      v
logical dataflow: steps                the program's own step nodes (HQL-0005)
      |
      v
stages and their dependencies          the planner
      |
      v
backend-specific execution plan        the backend
      |
      v
tasks                                  the backend's scheduler
      |
      v
execution, and the effects it has      the runtime, inside one job
```

A plan MUST give the value the `localhost` backend gives (HQL-0004). Within
that, a planner MAY fuse, split, reorder or distribute steps where doing so is
safe, and the companion note on [effects](02-effects.md) says where it is not.

## The plan on the `localhost` backend

This is what `hql plan` prints for it today, from the planner in
`src/backends/plan.rs`:

```text
backend localhost
line 1: source("orders")
  | filter    stage 0  in process    elementwise, pure
  | map       stage 0  in process    elementwise, pure
  | distinct  stage 0  in process    barrier, pure
  | sink      stage 0  in process    barrier, effectful
```

One job, one stage, one task. This is HQL-0005's trivial plan, and the record
leaves it as it is.

## A plan on a distributed backend

```text
job 1
  stage A   source, filter, map, distinct (partial)     pure, fused; per partition
    tasks A.0 .. A.63                                   one per input partition
        |
        | shuffle by customer_id
        v
  stage B   distinct (final)                            per key partition
    tasks B.0 .. B.15
        |
        v
  stage C   sink                                        effectful
    tasks C.0 .. C.15                                   one writer per partition
```

- `filter` and `map` are pure `Elementwise` steps, fused with the source into
  stage A: one traversal, no intermediate value.
- `distinct` is one step planned into two stages: a partial deduplication in
  stage A and the final one in stage B, across a shuffle. This is why a stage
  may hold part of a step, which HQL-0005 did not allow.
- `sink` is effectful. It is its own stage here so that its tasks can be
  retried, committed and counted apart from the pure work; a backend MAY fuse
  it into stage B when the endpoint's delivery semantics allow.
- A failed task A.17 is retried as a new task of stage A over partition 17.
  Nothing in the program changes.

The plan is one of many. Another backend pushes all of it into one SQL
statement: one stage, one task, run by the database.

## Each level against the records that use its word

| Level | Defined by | Earlier uses this record aligns |
| --- | --- | --- |
| Step | HQL-0005, "The six terms" | unchanged |
| Stage | HQL-0005, "A stage is an annotation on steps" | widened to part of a step; plans are graphs of stages |
| Task | this record | HQL-0036 "retrying failed execution tasks"; HQL-0005 "parallel, 8 jobs" means eight tasks |
| Job | this record | HQL-0005 "distributed as jobs" and "job units" mean tasks |

The discussion this record comes from proposed *work unit* for the
planner's region. HQL-0005 already reserves *stage* for exactly that, and
gives each term one meaning, so *work unit* is not adopted. Should a later
record prefer *work unit*, it renames *stage* everywhere, not beside it.

## Why the step is not a "job unit"

A step exists when HQL runs in process with no job orchestration at all, and
a name like *job unit* would make it sound like a subordinate part of a job.
The step is the program's unit; the job is the runtime's, and the plan
between them may relate the two in any way that preserves the program's
value.
