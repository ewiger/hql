# HQL-0055: local evaluation and scheduled execution

Companion to [HQL-0055](../README.md). It describes local evaluation today
and the responsibilities of future scheduled execution.

## Two ways to run

**Local evaluation.** The `hql` binary evaluates the checked program in
process: each pipeline is planned into one in-process stage. One program run
is one job in this vocabulary; task identities are not exposed by the local
runtime. Finite collections pass through steps in turn; streams produce
values on demand. This applies to every program today, including those that read from and write to
PostgreSQL, Kafka, NATS or a lakehouse: the endpoint is remote, the
evaluation is local.

**Scheduled execution (design direction).** A backend would run a plan's
stages as tasks on workers it manages or through an engine's scheduler.
The nearest case today is a read or a mutation over a Spark endpoint
(HQL-0049 and HQL-0051): HQL
sends one SQL statement, and the cluster plans and schedules that statement
using its own physical plan, out of HQL's sight. One submitted statement
may involve multiple engine stages, tasks and jobs.

**Delegated planning.** A future SQL backend could translate HQL's logical
steps into a query with CTEs and submit it to Databricks. At that handoff,
HQL has selected the engine and the translated computation. Spark still
plans the physical operators and schedules execution. A backend MAY own
those choices directly or delegate them; either way it owes HQL the same
observable behavior. The [four levels](01-levels.md) describe this boundary.

Backend configuration and execution context select the strategy (HQL-0025).
The checked program retains its meaning under each supported strategy.

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

Each layer has a responsibility for completion visible to the program:

```text
program     says which effectful autes happen, and in what order
planner     preserves effect dependencies and transaction scope in the physical plan
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

Pure tasks may be retried or duplicated when the backend can preserve
their results and observable failures. Effectful tasks additionally require
the delivery and transaction guarantees above.

## Bounded and unbounded jobs

- A **bounded** job ends when its sources end and its effects are committed:
  a batch load, a report, a migration.
- An **unbounded** job runs over a `Stream` that does not end: a consumer of a
  Kafka topic or a NATS subject feeding a sink. It ends on a signal, a drained
  source or a policy; which, and how a job's end commits its last effects, is
  an open question of the record.
- In this proposal, a `sink` gives `Step` when its input ends and its write
  completes. An unbounded input keeps it active until an end condition is
  reached. Other work in the same job may still remain. Progress reporting
  during that write belongs to the runtime; its interface remains open.

## Scope of this proposal

The proposal retains local evaluation, the `localhost` planner and
`hql plan`. Its execution vocabulary gives future backend specifications
a shared starting point. A graph IR, scheduler and public `Stage`, `Task`
or `Job` types are outside this proposal.
