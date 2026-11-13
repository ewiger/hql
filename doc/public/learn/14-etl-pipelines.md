# 14. ETL pipelines

Every data platform has a language for the wiring. Data Factory wires
activities in JSON, PySpark in a DataFrame API, ksqlDB in SQL over topics,
Kafka Streams in Java, dbt in SQL models with a dependency graph around them.
Each describes the same four things: where data comes from, what is done to
it, where it goes, and when. They differ in where a mistake is found: in a
validator before deployment, in a cluster at run time, or in a dashboard the
next morning.

HQL's claim is that the wiring is a program like any other in this book. A
pipeline is one `.hql` file. What it reads and writes are endpoint ids that
configuration binds to systems. Its types are checked before anything
connects. And the same text runs over a subject that never ends and over the
archive of what the subject once carried.

Part I was about the steps: what each one takes, what it gives, and the type
that joins them. This part is about the pipeline the steps form, and the
systems a pipeline connects. This chapter is the frame: what a pipeline is,
when it is checked, what a step may and may not do, and what stays outside
the file. The five after it follow one shop's orders through every kind of
system a pipeline reaches, and chapters [19](19-oltp/index.md) and
[20](20-olap/index.md) are the shop's two sides, the transactions as they
happen and the history they leave.

## A pipeline language

An HQL program is values flowing through typed steps. `|` is the operator
that forms a **step**: the value on its left becomes the input of the
operation on its right, and the result is the input of the next. A step
invokes exactly one **operation**, `filter`, `sort`, `map`, or one
**presentation**, `json`, `markdown`:

```hql
[3, 1, 2] | filter(n => n > 1) | sort | map(n => n * 10)
```

```text
[20, 30]
```

A dot selects a field or calls a function on a value. `s.trim()` is a
receiver call, and `b.title` a field read. Inside a pipeline step, these
expressions compute the values that its operation uses:

```hql
["Dune ", " Emma"] | map(s => s.trim().upper())
```

```text
["DUNE", "EMMA"]
```

The pipes describe the logical steps and their data dependencies. The dots
express computations within those steps. A planner uses that checked
program to choose physical execution stages; several steps may share a
stage. The companion [Logical steps and physical plans](14-planning-and-backends.md)
follows this distinction through local, parallel and SQL execution.

## Three phases

A program is parsed, then checked, then executed, and a diagnostic is headed
by the phase that wrote it: `parse failure` and `check error` before anything
runs, `execute failure` after ([chapter 12](12-errors/index.md)). Checking resolves every
name, import and module, infers and checks every type, and verifies every
trait implementation; a program the checker rejects has no meaning, and no
step of it executes. `hql check` runs the first two phases alone and prints
the type the third would produce:

<!-- run: check -->
```hql
[3, 1, 2] | filter(n => n > 1) | sort | map(n => n * 10)
```

```text
List<Int>
```

Execution is a strategy a **backend** chooses. `localhost`, the one backend
the current release has, evaluates the checked program in process. Finite
collections pass through each step in turn; streams produce values on
demand. A future backend that compiles a program or runs it over many
machines receives the same checked program. For the work it supports, it
must compute the value `localhost` computes, at the same type, with the
same failure where `localhost` fails. Unsupported work is a planning
refusal before execution. Backend selection belongs to configuration,
and `hql config` reports it.

## Logical steps and effects

A step is a logical unit of computation, established before planning. Its
operation defines the transformation from input to result. Physical planning
chooses how to perform that transformation, subject to its data dependencies,
shape and effects.

An **effectful** operation interacts with state outside the evaluation:
`source` reads, `target` and `sink` write, and a logger's `info` emits a
record. Planning preserves the order of effects that the program orders and
the scope of each transaction. Repeating an effect requires a delivery or
transaction guarantee that permits the repeat. A backend that cannot honor
the program's contract refuses it before execution.

A **pure** step may be rewritten as its shape and dependencies permit, while
preserving results, sequence order and failures. A step that processes each
value alone, `filter`, `map`, may run over separate parts of the input; a
step such as `sort`, `group`, `sum` or `size` needs coordination to produce
its complete result. The reading list of chapter 21 has three steps:
`filter` tests each card, `sort` orders the retained cards, and `map` turns
each card into its title. **MapReduce** is one possible execution strategy
for suitable steps: work on parts, then combine the results. The planner
chooses traversals and intermediate storage while preserving the pipeline's
result. `collect(n)` and `emit` change
the shape of what flows, batches of values or their members, and `tee`
branches a pipeline without ending it; each of them is a step like any other,
and the type says what the next step may ask.

## The command line

The smallest pipeline is a shell pipe. `hql '<program>'` reads standard input
with `source(stdin, Json)`, writes its result as `jq` or `awk` would, and
names no input or output of its own: input is `source` or `tap` in the
program, output is `target` or `sink` in the program, or the result, presented
by `--to`. The [command-line page](09-io/command-line.md) of chapter 9 shows
each flag; one program there reads a log as it is written, appends its errors
to a file and prints every message, with no orchestration around it. The
shell connects the processes, and each `hql` process evaluates its program
with the `localhost` backend.

## Streaming and batching

A platform's second vocabulary is about how a pipeline runs. A **batch**
job reads an input that is all there, computes over the whole of it, and
writes a result; it ends. A **streaming** job reads values as they arrive,
computes over each, and writes as it goes; it does not end. The same
transformation is often wanted both ways, and most platforms have two
engines, two APIs or two deployment modes for it.

In HQL the difference is the type of the input, and the four steps of
[chapter 9](09-io/index.md) carry it. `source` gives a `Seq`, which ends;
`tap` gives a `Stream`, which may not. `target` writes one completed value;
`sink` writes each value as it arrives. A transformation written over a
`Seq` is a batch job, and over a `Stream` the same text is a streaming job,
and the checker says which steps each allows. The revenue of the shop's
orders, over a table that ends:

<!-- define: Event -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}
```

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
orders : Seq<Event> = source("ledger")
orders | filter(o => o.kind == "order_placed") | map(o => o.amount) | sum
```

```text
185.74
```

and over a topic that does not:

<!-- run: --vault examples/pipelines/shop -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Stream<Event> = tap("order_events")
events | filter(o => o.kind == "order_placed") | map(o => o.amount) | sum
```

```text
check error[type-mismatch]: `sum` needs a finite input, and Stream<Decimal> may not end; bound it first with `take(n)`, or batch it with `collect(n)`
  --> <expression>:10:71
   |
10 | events | filter(o => o.kind == "order_placed") | map(o => o.amount) | sum
   |                                                                       ^^^
```

A sum over a stream has no moment to be complete, so it is refused before
the program runs, and the error names the two ways out. `take(n)` bounds the
stream and makes a batch of its first `n` values. `collect(n)` keeps the
stream and makes it a stream of batches, which is the **micro-batch** of
Spark Structured Streaming: inside each batch every whole-input step
applies, and the result is one value per batch, written as each closes:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
events : Stream<Event> = tap("events")
events
| collect(4)
| map(batch => batch | filter(o => o.kind == "order_placed") | map(o => o.amount) | sum)
| sink(stdout, Json)
```

```text
170.49
15.25
```

`tap` over the events file gives the six orders as a stream that ends after
the last, so this program is the streaming job run against recorded input.
That is the other half of the rule: an input that ends can be read either
way, `source` as a batch or `tap` as a stream, and a program written for the
stream is tested over a file and replayed over an archive with no change
but the id it reads, which [chapter 19](19-oltp/index.md) relies on.

The two vocabularies are independent. Streaming and batching say how a
pipeline runs, one value at a time or all of them at once. **OLTP** and
**OLAP**, chapters [19](19-oltp/index.md) and [20](20-olap/index.md), say
what the data is for, the transactions as they happen or the history they
leave. A transactional pipeline is usually streaming and an analytical one
usually batch, and the exceptions are everyday: an analytical table filled
as events arrive, a transactional table backfilled from an archive. The
types let a program say which it is without the platform having a word for
the combination.

## The shop

A small shop sells to three customers, ada, grace and linus. Each thing that
happens to an order is an **event**: placed, shipped, or a payment that
failed. On the first day the events are one JSON file, and the shop's whole
data platform is a pipeline over it:

<!-- run: --vault examples/pipelines/shop -->
```hql
source("events")
| filter(e => e.kind == "order_placed")
| map(e => e.customer)
| distinct
```

```text
["ada", "grace", "linus"]
```

`source("events")` names no file. The shop's `hql.toml`, in
[`examples/pipelines/shop/`](https://github.com/ewiger/hql/tree/main/examples/pipelines/shop),
declares `events` as a `json` endpoint, and the program names the id. That
one indirection is what lets the rest of Part II happen without the program
changing shape: as the shop grows, the events move into a table, then onto
a topic, then onto a subject a stream keeps, then into a lakehouse, and at
each move the configuration names the new system and the pipeline keeps its
steps.

| Chapter | The shop's events are in | Read by | Written by |
| --- | --- | --- | --- |
| [15. Endpoints](15-endpoints.md) | anywhere: the model every system shares | `source`, `tap` | `target`, `sink` |
| [16. Tables and topics](16-tables-and-topics.md) | a SQLite table, a PostgreSQL table, a Kafka topic | `source` for a table, `tap` for a topic | `target`, `sink` |
| [17. Subjects and the lakehouse](17-subjects-and-the-lakehouse.md) | NATS subjects, a JetStream stream, Delta tables | `tap` for a subject, `source` for a table | `sink`, `target` |
| [18. APIs, webhooks and stores](18-apis-webhooks-and-stores.md) | a REST API, a webhook, an object store, as design | `source`, `tap` | `target`, `sink` |
| [19. OLTP](19-oltp/index.md) | the transactions as they happen, all of the above in one file | `tap` | `sink` |
| [20. OLAP](20-olap/index.md) | the history they leave: bronze, silver and gold tables | `source` | `target` |

Every system in chapters 16, 17, 19 and 20 is in the current release. The
programs that reach a server are checked in this book, against the shop's
configuration and with no connection made, and run against real servers in
the [hql-demo repository](https://github.com/ewiger/hql-demo), whose output
is the output the book shows. A checked program is shown with what
`hql check` prints, the type its result would have, and a pipeline that
ends in `target` or `sink` has the type `Unit`: its result is nothing,
because its work is the writing.

**A job is one run of a program's plan.** An `.hql` file defines the program;
each invocation starts another run, which may include several pipelines.
Its steps describe transformations, reads and writes. Configuration names
the environment variables that supply credentials and selects the backend.
The schedule determines when the program runs. `hql check` reads the
configuration without connecting to endpoints, so a deployment can check
the program before running it.

A job's own settings, the customer a report is for or the day it covers,
come from the invocation that runs it. `hql --arg customer=ada run
report.hql` passes one, and the program reads it with `env.arg` from
`import io.env`, as text, with a default where it is not passed
([environment and arguments](09-io/environment.md)):

<!-- run: --vault examples/pipelines/shop --arg customer=ada -->
```hql
import io.env

type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

customer = env.arg("customer") | unwrap_or("grace")
orders : Seq<Event> = source("ledger")
orders
| filter(o => o.kind == "order_placed" and o.customer == customer)
| map(o => o.amount)
| sum
```

```text
135.75
```

## Partitions, schedules and backends

The program can declare logical partitioning. Scheduling and backend
selection are configuration around it.

A **partition** is the logical unit of ownership. `partition(by = e =>
e.customer)` is in the current release: it states that the orders of one
customer belong together, keeps every value and its type, and on
`localhost` changes nothing else. It names no worker and no Kafka partition.
It is what a backend reads to know where a per-customer `group` may run
without the parts having to meet.

!!! warning "Design only"

    The rest of this section describes functionality that is not in the
    current release. The configuration examples illustrate design direction;
    names and keys may change before it ships.

A **schedule** is configuration beside the endpoints, `[jobs.report] run =
"report.hql"` with an `at`, or a trigger that is itself a source: a subject
whose messages each start a run. The file does not know when it runs.

A **backend** supplies an execution strategy for the checked program.
Future backends could use parallel workers in one process, translate a
supported computation into SQL, or distribute it across a cluster. Each
must preserve the result, types, sequence order, failures and effects of
local evaluation. The companion [Logical steps and physical plans](14-planning-and-backends.md)
explains the planning handoff, including Spark SQL and CTEs on Databricks.

```toml
[execution]
backend = "spark"
```

This illustrates future execution configuration. The `spark` endpoint kind
in chapter 17 already identifies a table the program reads and writes.
Backend selection would additionally choose where pipeline computation
runs, while keeping the checked program's meaning.

## What each of them wires

| | Sources and sinks | Transformations | Where a mistake is found |
| --- | --- | --- | --- |
| Data Factory | linked services and datasets, in JSON | activities, each a box | at validation of the JSON, or in the run's monitor |
| PySpark | readers and writers, in code | DataFrame operations | when the job runs, at the operation that fails |
| ksqlDB | topics as streams and tables | SQL over them | when the statement is submitted, or in the records it skips |
| dbt | models over a warehouse | SQL, with tests after | when the model builds, or when its tests run |
| HQL | ids bound in `hql.toml` | the steps of this book | when the file is checked, before anything connects |

The columns are the same four things. What HQL adds is a type for each
arrow: stream against table, finite against live, a declared record against
a configured boundary, a contract against what a producer actually sent,
each checked where the file is written, so that the file that passed is the
file that runs.

## What is next

[Chapter 15](15-endpoints.md) looks at what `"events"` names: an endpoint,
the one model behind a file, a table, a topic and a webhook.
