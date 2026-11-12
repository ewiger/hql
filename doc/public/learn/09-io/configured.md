# Sources and targets by id

A program that names a file path is tied to one machine's files. A vault
declares its sources in `hql.toml` instead, each under an id, and a program
names only the id. The field station of the
[data sources examples](https://github.com/ewiger/hql/tree/main/examples/sources)
is such a vault, and the programs on this page run with
`--vault examples/sources/station`:

```toml
[sources.journal]
kind = "vault"
root = "journal"

[sources.counts]
kind = "csv"
path = "data/counts.csv"

[sources.sightings]
kind = "json"
path = "data/sightings.json"
```

<!-- run: --vault examples/sources/station -->
```hql
journal = source("journal")
journal.cards | map(c => c.name)
```

```text
["kite", "owl", "wren"]
```

A vault is a folder of documents, each a card with a name, the subject of
[chapter 16](../16-a-vault.md); `journal.cards` is the cards of that one.
Here it is one source among others, read by its id like the rest.

## The kind states the type

`source("journal")` is a `Vault`, with its own `cards`. `source("counts")` is
a `Seq<Data>`, a row per element, and a `json` source is the elements of its
file's one array:

<!-- run: --vault examples/sources/station -->
```hql
source("counts")
```

```text
[{count: "2", species: "wren"}, {count: "1", species: "owl"}, {count: "1", species: "kite"}]
```

<!-- run: --vault examples/sources/station -->
```hql
source("sightings") | map(s => s.species)
```

```text
["wren", "owl", "kite"]
```

The kinds are `vault`, `csv`, `json`, `json_lines`, `yaml`, `sqlite`, one
table of a database file, `postgres`, one table of a PostgreSQL database, and
`kafka`, one Kafka topic; the last two are the subject of
[topics and tables](#topics-and-tables). A path resolves against the directory of
`hql.toml`. The type comes from the configuration, so checking a program
reads `hql.toml` and opens nothing else.

A source read as a declared record decodes each element as the record, as a
file does, and `tap("id")` gives a finite source one element at a time, as a
`Stream`:

<!-- run: --vault examples/sources/station -->
```hql
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("counts")
tallies | map(t => t.count) | sum
```

```text
4
```

## The id is a literal

The type of a source is known only from its configuration, so the id is
written in the program as a literal, and a connection string is not an id:

<!-- run: --vault examples/sources/station -->
```hql
id = "counts"
source(id)
```

```text
check error[type-mismatch]: `source` names a configured source by a literal id, `source("field_notes")`: a computed id would leave its type unknown until evaluation
 --> <expression>:2:8
  |
2 | source(id)
  |        ^^
```

<!-- run: --vault examples/sources/station -->
```hql
source("postgres://db/birds")
```

```text
check error[type-mismatch]: "postgres://db/birds" is not a source id: connection details belong in configuration, `[sources.<id>]` in hql.toml, and a program names the id
 --> <expression>:1:8
  |
1 | source("postgres://db/birds")
  |        ^^^^^^^^^^^^^^^^^^^^^
```

An id the configuration does not declare is a name error that names the
nearest one:

<!-- run: --vault examples/sources/station -->
```hql
source("count")
```

```text
check error[unknown-name]: no source `count` is configured; did you mean `counts`?
 --> <expression>:1:8
  |
1 | source("count")
  |        ^^^^^^^
```

## The schema boundary

The names and types an external system uses are facts about that system, not
about the program's types, so they are configured beside the source. The
station's ledger is a CSV whose headers are `Species Name`, `Total` and
`Observer`. `[sources.<id>.fields.<field>]` maps a field of the record a
program reads to its external field, by `name` for a column or a key, and by
a dotted `path` for a key below the top level of a JSON or YAML document:

```toml
[sources.ledger]
kind = "csv"
path = "data/ledger.csv"

[sources.ledger.fields.species]
name = "Species Name"

[sources.ledger.fields.count]
name = "Total"

[sources.profiles]
kind = "json"
path = "data/profiles.json"

[sources.profiles.fields.id]
name = "user_id"

[sources.profiles.fields.name]
path = "profile.display_name"
```

<!-- run: --vault examples/sources/station -->
```hql
type Profile {
    id   : String
    name : String
}

profiles : Seq<Profile> = source("profiles")
profiles | map(p => p.name)
```

```text
["Ada", "Grace"]
```

A field's table also takes `type`, the external type where the format's
default is not it; `absent`, the external values read as an absent optional
field, an empty cell in CSV unless written; and `write = false` for a field a
write leaves out.

`strictness` says what a read does with an external field no field of the
record reads. It is `compatible` unless written: the field is passed to the
type, so a closed record does not decode a ledger with an `Observer` column.
Under `open` the field is not read, under `warn` it is not read and is
reported, and under `strict` it is an error:

<!-- run: --vault examples/sources/station -->
```hql
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("ledger")
tallies | map(t => t.count) | sum
```

```text
execute failure[io-failure]: source `ledger`: document 1 does not decode as Tally: field `Observer`: Tally declares no such field
 --> <expression>:6:24
  |
6 | tallies : Seq<Tally> = source("ledger")
  |                        ^^^^^^^^^^^^^^^^
```

```toml
[sources.ledger_open]
kind = "csv"
path = "data/ledger.csv"
strictness = "open"

[sources.ledger_open.fields.species]
name = "Species Name"

[sources.ledger_open.fields.count]
name = "Total"
```

<!-- run: --vault examples/sources/station -->
```hql
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("ledger_open")
tallies | map(t => t.count) | sum
```

```text
4
```

`hql boundary <file> <id> --as <Type>` prints a source's effective boundary,
each property with the layer it comes from, the format's default or the
configuration:

```text
$ hql --vault examples/sources/station boundary tally.hql ledger --as Tally
source ledger (csv), read as Tally; strictness compatible, mode replace
species : String
  external Species Name    configuration
  type     text            format
  position 1               format
  read     true            format
  write    true            format
  absent   empty, missing  format
count : Int
  external Total           configuration
  type     text            format
  position 2               format
  read     true            format
  write    true            format
  absent   empty, missing  format
```

## Writing by id

A configured source is also a target. `xs | target("id")` writes a
`Table<R>`, or a `List<R>` or `Seq<R>`, of a declared record `R` through the
same boundary: each field under its external name, in position order, a
field with `write = false` left out. The kind fixes the format, so a format
beside the id is a type error, and a vault is read and never written:

<!-- run: check --vault examples/sources/station -->
```hql
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source("counts")
tallies | filter(t => t.count > 1) | target("counts")
```

```text
Unit
```

<!-- run: --vault examples/sources/station -->
```hql
source("counts") | target("counts")
```

```text
check error[type-mismatch]: `target("counts")` writes rows of a declared record, and Data is not one
 --> <expression>:1:20
  |
1 | source("counts") | target("counts")
  |                    ^^^^^^^^^^^^^^^^
```

<!-- run: --vault examples/sources/station -->
```hql
source("counts") | target("counts", Csv)
```

```text
check error[type-mismatch]: a configured target's format is its kind's, set in its configuration; name none
 --> <expression>:1:20
  |
1 | source("counts") | target("counts", Csv)
  |                    ^^^^^^^^^^^^^^^^^^^^^
```

The source's table takes `mode`, how a write treats what the target already
holds. It is `replace` unless written:

| `mode` | A file | A SQLite table |
| --- | --- | --- |
| `create` | written where none exists; an existing file is an evaluation failure | created where none exists; an existing table is an evaluation failure |
| `append` | the rows or documents added after the ones held; created where none exists | rows inserted; the table created where none exists |
| `replace` | rewritten whole | dropped and created again with the rows written |

Reading a target the same evaluation has replaced is an evaluation failure,
as for a file, and a document being rendered writes nothing
([effects and what a host allows](hosts.md)).

## Topics and tables

A `postgres` source is one table of a PostgreSQL database, and a `kafka`
source is one Kafka topic. The connection string of a database may hold a
password, so the configuration names the environment variable that holds it,
`dsn_env`; a topic's brokers are written as `brokers` or named the same way
by `brokers_env`:

```toml
[sources.orders]
kind = "postgres"
dsn_env = "DEMO_DSN"
table = "orders"
order = ["id"]

[sources.events]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "events"
idle_ms = 2000
```

Checking a program reads this configuration and connects to neither system.
The variables are read, and the connections made, when a source is read or a
target written.

**A table ends.** `source("orders")` is a `Seq` of rows, read through the
schema boundary as a `sqlite` table is: the columns are compared with the
record before any row is decoded, and only the columns the record reads are
selected. A `numeric` column reads as a `Decimal` with its digits, a `jsonb`
column as the tree it holds, and a `NULL` as an absent field:

<!-- run: skip -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Seq<Event> = source("orders")
events
| filter(e => e.kind == "order_placed")
| group(by = e => e.customer)
| map(g => {customer: g.key, total: g.items | map(e => e.amount) | sum})
```

```text
{{customer: "ada", total: 135.75}, {customer: "grace", total: 42.00}, {customer: "linus", total: 7.99}}
```

`target("orders")` writes a `Seq` or a `Table` of a declared record in one
transaction. A table that does not exist is created, a column per field:
`BIGINT` for an `Int`, `DOUBLE PRECISION` for a `Float`, `NUMERIC` for a
`Decimal`, `BOOLEAN` for a `Bool`, `JSONB` for a list or a record, and `TEXT`
otherwise. Under `mode = "replace"` the rows the table holds are removed and
its definition, indexes and grants are kept.

**A topic does not end.** `tap("events")` is a `Stream`, one JSON document
per message, decoded as the element type. `source("events")` is a type
error, since a `Seq` is a value that ends:

<!-- run: skip -->
```hql
source("events")
```

```text
check error[type-mismatch]: `events` is a `kafka` source, read by `tap` as a `Stream`, not by `source` as a `Seq`
 --> <expression>:1:8
  |
1 | source("events")
  |        ^^^^^^^^
```

The stream starts at the earliest message the topic keeps, or at the next
one under `from = "latest"`. Without `idle_ms` it waits for messages without
end; with it, the stream ends once no message has arrived for that many
milliseconds.

**`sink` writes as values arrive.** `sink("events")` writes each value as one
message, and `sink("orders")` each row as it arrives, committed one at a
time. A stream has no last value for `target` to wait for, so moving a topic
into a table is a `tap` and a `sink`:

<!-- run: skip -->
```hql
events : Stream<Event> = tap("events")
events | sink("orders")
```

and moving a table into a topic is a `source` and a `sink`:

<!-- run: skip -->
```hql
orders : Seq<Event> = source("orders")
orders | sink("events")
```

`sink` takes no format beside an id, and writes to a `kafka` or a `postgres`
source only; a file or an SQLite table is written whole by `target`.
