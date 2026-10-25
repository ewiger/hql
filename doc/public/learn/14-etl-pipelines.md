# 14. ETL pipelines

!!! warning "Design only"

    This chapter describes functionality that is not in the current release.
    It is written as if it existed: apart from the diagnostics the current
    binary already gives, these programs do not run, and names, kinds, types
    and outputs may change before any of it ships. It is a story about where
    the pipeline of chapter 13 is going, told in one file.

Every data platform has a language for the wiring. Data Factory wires
activities in JSON, PySpark in a DataFrame API, ksqlDB in SQL over topics,
Kafka Streams in Java, dbt in SQL models with a dependency graph around them.
Each describes the same four things: where data comes from, what is done to
it, where it goes, and when. They differ in where a mistake is found: in a
validator before deployment, in a cluster at run time, or in a dashboard the
next morning.

HQL's claim is that the wiring is a program like any other in this book. A
pipeline is one `.hql` file; its sources and targets are ids that
configuration binds to systems; its types are checked before anything
connects; and the same text runs over a queue that never ends and over the
archive of what the queue once carried.

## A pipeline is a file

A field station publishes bird sightings from its sensors to a Kafka topic.
A Postgres database holds the sites and receives daily counts. Alerts go to a
second topic, and everything read is archived as JSON Lines. The station's
`hql.toml` names each of those systems once:

```toml
[sources.sightings]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "sightings"
group = "station-nightly"

[sources.sites]
kind = "postgres"
dsn_env = "STATION_DSN"
table = "sites"

[sources.daily_counts]
kind = "postgres"
dsn_env = "STATION_DSN"
table = "daily_counts"
mode = "append"

[sources.alerts]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "alerts"

[sources.archive]
kind = "json_lines"
path = "archive/sightings.jsonl"
mode = "append"
```

The pipeline, `nightly.hql`, names only the ids:

<!-- run: skip -->
```hql
type Sighting { species : String, site : String, seen : String, at : DateTime }
type Site     { site : String, region : String, protected : Bool }
type Count    { day : Date, region : String, species : String, sightings : Int }

sites : Seq<Site> = source("sites")

tap("sightings")
| Sighting
| tee archived = sink("archive")
| join(sites, on = (s, p) => s.site == p.site)
| map(j => {species: j.left.species, region: j.right.region, protected: j.right.protected, at: j.left.at})
| tee alerts =
    filter(x => x.protected and x.species == "barn owl")
    | map(x => {region: x.region, at: x.at, text: fmt("barn owl in protected {}", x.region)})
    | sink("alerts")
| window(1d, by = x => x.at)
| map(w => w
    | group(by = x => {region: x.region, species: x.species})
    | map(g => Count(day = w.start.date, region = g.key.region, species = g.key.species, sightings = g.items | size)))
| emit
| sink("daily_counts")
```

Read top to bottom, it is the dataflow diagram a platform would draw: one
input, a lookup, two side outputs, a windowed aggregate, one main output.
There is no orchestration around it, no activity names, no cluster
configuration in the text. `hql check nightly.hql` reads `hql.toml`, resolves
every id to its kind and every kind to a type, checks the whole file, and
opens no connection.

## Ids, not connections

`source("sites")` and `tap("sightings")` name configuration. The id is a
literal, because the type of what it reads is known only from the
configuration, and a program whose source could be computed would have a type
nobody could check. Three refusals exist today, on the
[sources-by-id page](09-io/configured.md) of chapter 9: a computed id is a
type error, a connection string in place of an id is a type error that says
where connection details belong, and an id the configuration does not declare
is a name error naming the nearest one.

A binding can break, and the design is that it breaks at the earliest phase
that can see it. Checking sees configuration; evaluation sees the system:

| What is wrong | When it is found | What is said |
| --- | --- | --- |
| `source("sightngs")` | checking | name error: no source `sightngs` is configured; did you mean `sightings`? |
| `sites : Seq<Site> = source("sightings")` | checking | type error: `sightings` is a `kafka` source, read by `tap` as a `Stream`, not by `source` as a `Seq` |
| `source("sites")` read as `Seq<Count>` | checking | type error: the boundary of `sites` has no field `day`; `Count` declares one |
| `STATION_DSN` not set | evaluation, before the first row | evaluation failure: source `sites` reads its connection from `$STATION_DSN`, which is not set |
| a row whose `protected` is `"yes"` | evaluation, at that row | evaluation failure: source `sites`: row 14 does not decode as Site: field `protected`: "yes" is not a Bool |
| the broker unreachable | evaluation, before the first message | evaluation failure: source `sightings`: no broker at `$KAFKA_BROKERS` answered |

A secret is named by the environment variable that holds it and never
printed. A failure at evaluation names the source by its id, the system by
its configured name, and the row or message by its position, as a CSV
decode error does today; what it never does is continue past a row it could
not read and count it later as missing.

The configuration is also where a system's names meet the program's. The
`sites` table may call its region column `region_code`; the boundary of
chapter 9 maps it, `[sources.sites.fields.region] name = "region_code"`, and
the program keeps the name it reads. Renaming a column is a change to one
line of configuration, not to every pipeline that reads it.

## Streams in, tables out

`tap("sightings")` is a `Stream<Data>`, and `| Sighting` reads each message
as a declared record, so the rest of the file is typed. `source("sites")` is
a `Seq<Site>`: a table is finite, read once, replayable within the
evaluation. The join between them is the common shape of enrichment, a
stream against a lookup, and the type says why it is cheap: the finite side
is read whole, the live side passes through one message at a time, and no
message waits for another.

Two streams joined would wait for each other without end, and the checker
says so: `join` over two `Stream`s is refused until one of them is bounded by
a window. A window is `collect` by time rather than by count,
`window(1d, by = x => x.at)`, a `Stream<Seq<T>>` whose batches close when the
day does. Inside a batch every whole-input step of chapter 6 applies, which is
how the daily counts are a `group` and a `size` over an endless input.
`emit` lets the counts out one at a time, and `sink("daily_counts")` appends
each as a row.

The two `tee` branches write without ending the pipeline. The archive is
every sighting as it arrived, before enrichment; the alerts are a filter and
a `map`, written to a topic as messages. A `tee` branch is the fan-out a
platform draws as a diamond, and it is typed like everything else: a branch
that ended in `target` over the stream would be refused, because a stream may
never complete, and `sink` is what a stream is written with.

## From a batch to a stream, and back

A pipeline written for a live topic is a pipeline written for its history,
because the steps are the same and the type is the only difference. The
archive is a file, so `source("archive")` is a `Seq<Data>`; `stream` turns a
finite sequence into a stream that produces its elements in order and then
ends, so the nightly pipeline replays a day from the archive with the topic
replaced and nothing else changed:

<!-- run: skip -->
```hql
fn nightly(sightings: Stream<Sighting>) : Unit = …

source("archive") | Sighting | filter(s => s.at.date == Date(2026, 10, 24)) | stream | nightly
```

The other direction is as short. A stream becomes one file by bounding it
and writing the bound: `take(n)` for a count, `window` for a span,
`collect(n)` for batches of a size:

<!-- run: skip -->
```hql
tap("sightings") | window(1h) | emit | sink(file("last-hour.jsonl"), Json)
```

<!-- run: skip -->
```hql
source(file("examples/io/sightings.jsonl")) | take(2) | target(file("two.json"))
```

A backfill is the first program over the archive; a snapshot is the second
over the topic; a replay for a test is either over a file of recorded
messages, with no broker at all. The platform words, backfill, snapshot,
replay, name three ways to bound or unbound one pipeline, not three
pipelines.

## Rows that do not decode

A pipeline over a queue meets messages that are not what the type says. The
default is the strict one above: the first such message stops the
evaluation, naming it. A pipeline that must keep running routes them
instead. `Sighting?` reads each message as a `Result<Sighting, Invalid>`,
and `fork` splits a stream of results into its two sides without ending
either:

<!-- run: skip -->
```hql
tap("sightings")
| Sighting?
| fork
| tee rejected =
    errs()
    | map(e => {message: e.input, why: e.violations | map(v => v.message)})
    | sink("dead_letter")
| oks()
| nightly
```

The dead-letter queue is a `sink` like the others, and what reaches it is a
value: the message as it arrived and every violation at once, the way
[validation as a value](11-advanced-data-typing/validation-as-a-value.md)
reports them. Nothing is logged and lost; the rejected rows are a source
another pipeline reads.

## Partitions, schedules and backends

Three things a platform configures stay out of the file.

A **partition** is the logical unit of ownership: `partition(by = x =>
x.region)` states that values of one region belong together, so a windowed
group per region may run wherever that region's values are. It names no
worker and no Kafka partition; a backend maps the logical partitions onto its
own, and the local backend runs them one after another with the same result.

A **schedule** is configuration beside the sources, `[jobs.nightly] run =
"nightly.hql"` with an `at`, or a trigger that is itself a source, a topic
whose messages start a run. The file does not know when it runs.

A **backend** is the execution strategy of chapter 13: `localhost` in a
terminal, a distributed one in a cluster, chosen by `[execution] backend` or
`--backend`, and bound to compute what `localhost` computes. `hql check` is
the same under every backend, and it is what a deployment runs before it
deploys: the configuration, the types and the file, with no system touched.

## What each of them wires

| | Sources and sinks | Transformations | Where a mistake is found |
| --- | --- | --- | --- |
| Data Factory | linked services and datasets, in JSON | activities, each a box | at validation of the JSON, or in the run's monitor |
| PySpark | readers and writers, in code | DataFrame operations | when the job runs, at the operation that fails |
| ksqlDB | topics as streams and tables | SQL over them | when the statement is submitted, or in the records it skips |
| dbt | models over a warehouse | SQL, with tests after | when the model builds, or when its tests run |
| HQL, as designed | ids bound in `hql.toml` | the steps of this book | when the file is checked, before anything connects |

The columns are the same four things. What HQL adds is a type for each
arrow, so that stream against table, finite against live, declared record
against configured boundary, are checked where the file is written, and the
file that passed is the file that runs.

## What is next

Part III reads a different kind of data. [Chapter 15](15-a-vault.md) opens a
folder of documents and asks it the questions chapter 6 asked a table.
