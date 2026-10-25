# 13. Pipelines

Part I was about the steps: what each one takes, what it gives, and the type
that joins them. This part is about the pipeline the steps form, and the
systems a pipeline connects. Today it is one chapter, read as a frame for the
first part and a direction for the rest.

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

A dot is not a step. `s.trim()` is a receiver call, and `b.title` a field
read; both are expression evaluation inside the step that holds them, and
neither is a position in the pipeline:

```hql
["Dune ", " Emma"] | map(s => s.trim().upper())
```

```text
["DUNE", "EMMA"]
```

The two operators answer two questions. `|` says what happens to the whole
collection, one operation at a time, in the order written. `.` says what is
computed from one value, inside one of those operations. A reader finds the
shape of a program in its pipes, and the detail of each stage in its dots.

## Three phases

A program is parsed, then checked, then executed, and a diagnostic names the
phase that produced it: `syntax failure`, `name error` or `type error`
before anything runs, `evaluation failure` after. Checking resolves every
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
the current release has, evaluates the checked program step by step, each
step run to completion before the next, in memory. A backend that compiles a
program, or runs it over many machines, is bound to the same contract: it
accepts exactly what the checker accepts, and it computes the value
`localhost` computes, at the same type, with the same failure where
`localhost` fails. There is no syntax for choosing a backend, because a
program's answer does not depend on it; the choice is configuration, and
`hql config` reports it.

## The unit of execution

A step is the unit of execution. It runs over its whole input, and a backend
may not break it, reorder it against an effect, or run it twice. Two rules
follow.

An **effectful** step, one that reaches outside the program, `source`,
`target`, `sink`, a logger's `info`, runs once, where it is written, in
program order. A backend may plan how the pure steps between two effects are
fused or distributed; it may not move an effect, and a backend that cannot
run a program refuses it before running any part of it.

A **pure** step may be rewritten by its shape. A step that decides each value
alone, `filter`, `map`, may run over any part of the input in any order; a
step that needs the whole input, `sort`, `group`, `sum`, `size`, is where the
parts meet, and the pipeline holds nothing in between. The reading list of
chapter 14 is three such steps: `filter` decides each card on its own, `sort`
needs every card before it can place the first, and `map` turns each card into
its title. That alternation of per-value steps and whole-input steps is the
shape of **MapReduce**, and HQL keeps it visible in the pipeline: nothing is
stored between steps, and nothing is a loop. `collect(n)` and `emit` change
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
query is the dataflow, and the shell is the simplest backend of all.

## Connecting systems

!!! warning "Design only"

    This section describes functionality that is not in the current release.
    It is written as if it existed: these programs do not run against the
    current binary, and names, types and outputs may change before they ship.

The systems a data platform joins, queues, databases, services, scheduled
jobs, are endpoints and configured sources to a pipeline. Each is reached by
`source`, `tap`, `target` and `sink`, with the type saying whether it ends.

**Configured sources** keep connection details out of programs. A program
names an id; `hql.toml` names the system, and a secret is read from the
environment the configuration names, never written in either:

```toml
[sources.sightings]
kind = "postgres"
dsn_env = "BIRDS_DSN"
table = "sightings"
```

<!-- run: skip -->
```hql
type Sighting { species : String, site : String, seen : String }

sightings : Seq<Sighting> = source("sightings")
sightings | group(by = s => s.species) | map(g => {species: g.key, seen: g.items | size})
```

A relational table is a `Seq` of rows read through the schema boundary of
chapter 9; a write by id is `target("sightings")`, in the `mode` the
configuration states. A connection string in a program is a type error today,
and stays one.

**Messaging systems** such as Kafka and NATS are live endpoints. A topic is
read by `tap`, as a `Stream`, and written by `sink`, one document per value;
`collect(n)` batches what is written, and `partition(by = key)` states which
values belong together, a logical unit a backend may map onto the system's
own partitions without the program naming them:

```toml
[sources.events]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "sightings"
```

<!-- run: skip -->
```hql
tap("events")
| filter(e => e.seen == "night")
| partition(by = e => e.site)
| sink("alerts")
```

**Network endpoints**, `http`, `websocket` and `socket` under `import net`,
are in the current release, on the [network page](09-io/network.md). An HTTP
resource read once is a `Seq`; a socket is a `Stream`.

**A pipeline file is a job.** What a data platform calls a job, a notebook or
a workflow, sources, transformations, sinks and a schedule, is an `.hql` file
whose steps are the transformations and whose `source` and `sink` steps are
its edges; the schedule, the credentials and the backend are configuration
around it, and `hql check` reads the configuration and opens nothing. The
same file runs on `localhost` from a terminal and, as design direction, on a
backend that distributes its pure steps, with the same answer.

## What is next

Part III reads a different kind of data. [Chapter 14](14-a-vault.md) opens a
folder of documents and asks it the questions chapter 6 asked a table.
