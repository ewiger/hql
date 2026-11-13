# HQL-0055: local evaluation and scheduled execution

Companion to [HQL-0055](../README.md). It contrasts the two ways a program
runs today and later, and says which level owns which concern.

## Two ways to run

**Local evaluation.** The `hql` binary evaluates the checked program in
process: one job, one stage, one task, steps evaluated one after another.
This is every program today, including those that read from and write to
PostgreSQL, Kafka, NATS or a lakehouse: the endpoint is remote, the
evaluation is local.

**Scheduled execution.** A backend receives a plan and runs its stages as
tasks on resources it schedules: Spark executors, a database's own planner,
independently scheduled workers. Nothing in HQL does this yet. The nearest
case today is a read or a mutation over a Spark endpoint (HQL-0049): HQL
sends one SQL statement, and the cluster plans and schedules that statement
as its own job, out of HQL's sight.

The program does not say which. Its meaning is the same under both, and the
choice of backend is configuration and execution context (HQL-0025), not
syntax.

## Who owns what

| Concern | Level | Owner |
| --- | --- | --- |
| What is computed, and in what order effects happen | step | the program |
| Which steps fuse, where a shuffle falls | stage | the planner |
| Partition assignment, parallelism, retries, a task's identity | task | the backend's scheduler |
| Start, progress, end, cancellation, the run's diagnostics | job | the runtime |

- A task's identity and retry policy are runtime concerns. They are never a
  property of HQL syntax, and a program MUST NOT depend on how many tasks ran.
- The run-scoped diagnostic channel of HQL-0045 belongs to the job: every
  task's diagnostics reach the one channel of the job that ran it.
- A transaction (HQL-0051) is an effect constraint the planner carries into
  the job; its attempts are the program's retries, distinct from a backend's
  task retries.

## The aute between the program and the scheduler

The four levels are owned by four parties; the effectful autes are what they
agree on.

```text
program     says which effectful autes happen, and in what order
planner     places every effectful aute in a stage, never splitting or reordering it
scheduler   runs tasks; a task MUST NOT repeat an effect it completed unless
            the effect is idempotent, the endpoint allows the repeat, or the
            repeat is a transaction's attempt
runtime     reports the job's completed autes: progress, diagnostics, end
```

Locally, an aute is complete when its statement or step returns. On a
scheduled backend an effectful aute is complete when the backend confirms the
effect: a commit, an acknowledged publish, a file renamed into place. A
backend that cannot confirm an effect cannot claim it happened, and its task
has not completed.

A stage of pure steps completes no effectful aute. Its tasks may be lost,
retried and duplicated without the program observing any of it.

## Bounded and unbounded jobs

- A **bounded** job ends when its sources end and its effects are committed:
  a batch load, a report, a migration.
- An **unbounded** job runs over a `Stream` that does not end: a consumer of a
  Kafka topic or a NATS subject feeding a sink. It ends on a signal, a drained
  source or a policy; which, and how a job's end commits its last effects, is
  an open question of the record.
- A `sink` over an unbounded stream gives `Step` only when the job ends. Until
  then its progress is a property of the job, read from the runtime, not a
  value of the program.

## What 0.6.0 ships

Local evaluation, the `localhost` planner and `hql plan` as they are, and
the vocabulary. No `Stage`, `Task` or `Job` type, no plan IR, no scheduler.
The record exists so that the first scheduled backend arrives into names
that already have one meaning each.
