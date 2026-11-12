# HQL-0046: Kafka topics and PostgreSQL tables as configured endpoints

**Status**: accepted, implemented
**Created**: 2026-11-12
**Source**: [HQL-0012](../HQL-0012/README.md) (configured endpoints),
[HQL-0021](../HQL-0021/README.md) (endpoints, `source` and `tap`),
[HQL-0035](../HQL-0035/README.md) (the schema boundary and write modes),
the design direction of the pipelines and ETL chapters
(`doc/public/learn/14-pipelines.md`, `doc/public/learn/19-etl-pipelines.md`),
`src/kafka.rs`, `src/postgres.rs`, `src/endpoints.rs`, `src/extensions/io.rs`

## Abstract

Two kinds join `[endpoint.<id>]` in `hql.toml`. `kind = "postgres"` is one
table of a PostgreSQL database: a finite source, read by `source("id")` as a
`Seq` through the boundary, and written by `target("id")` in one transaction
or by `sink("id")` one row at a time. `kind = "kafka"` is one topic: a log
that does not end, read by `tap("id")` as a `Stream`, one JSON document per
message, and written by `sink("id")` or `target("id")`, one message per
value. `source("id")` of a topic is a type error. `sink` takes a configured
id, as `target` does. Connection details stay in configuration and secrets in
the environment.

## Motivation

- A data platform's systems are queues and databases; a pipeline file reaches
  them by id, as it reaches a file, so its types are known offline.
- `source` and `tap` already say whether documents end. A table ends; a topic
  does not. The two kinds are the first configured endpoints on each side of
  that line.

## Non-goals

- Consumer groups and committed offsets: a read starts where `from` says.
- TLS and SASL to either system.
- Message keys, headers and timestamps as values: a message is its value.
- `partition(by = …)` and its mapping onto a topic's partitions.

## Specification

### 1. Configuration

Each kind takes its own keys besides `kind`, `element`, `fields`, `mode` and
`strictness`.

- `postgres` MUST name `dsn_env`, the environment variable holding a
  libpq connection string, and `table`, schema-qualified where it holds a dot
  (`audit.events`). It MAY name `order`, the columns rows are read in; without
  it rows are read as the table is scanned.
- `dsn` is a secret key: written in the configuration it refuses the vault,
  naming `dsn_env`. A connection string may hold a password.
- `kafka` MUST name its brokers once, by `brokers` (`"host:9092,host2:9092"`)
  or by `brokers_env`, and `topic`. It MAY name `from`, `earliest` (the
  default) or `latest`, where a read starts in each partition, and `idle_ms`,
  a count above 0.
- A `postgres` or `kafka` endpoint takes no `*_env` key but its own.

```toml
[endpoint.orders]
kind = "postgres"
dsn_env = "DEMO_DSN"
table = "orders"
order = ["id"]

[endpoint.events]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "events"
idle_ms = 2000
```

### 2. Types

The kind states the type, read from the configuration at check time; checking
opens no connection.

- `source("id")` of a `postgres` endpoint is a `Seq` of the element, as for
  `sqlite`; `tap("id")` gives its rows as a `Stream`.
- `tap("id")` of a `kafka` endpoint is a `Stream` of the element.
- `source("id")` of a `kafka` endpoint MUST be a type error: `` `events` is a
  `kafka` endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq` ``.
- `sink(xs, "id")` checks that the id names a `kafka` or `postgres` endpoint and
  that no format is named. A topic takes values of any type; a table takes
  rows of a declared record. Any other kind is refused, naming `target`.
- `target(xs, "id")` of a `kafka` endpoint takes a completed collection of any
  element; a stream is refused, as for every target.

### 3. Reading a table

- The table's columns are read before any row and checked against the
  boundary; strictness applies to an unmapped column there.
- Only the columns the boundary reads are selected.
- Each value crosses as text and is converted by the column's type:
  `int2`/`int4`/`int8` to an integer, `float4`/`float8` to a float, `numeric`
  to a decimal with its digits, `bool` to a boolean, `json`/`jsonb` to the
  tree it holds, every other type to its text. `NULL` is absent by default.

### 4. Writing a table

The boundary's PostgreSQL types are `BIGINT` (Int), `DOUBLE PRECISION`
(Float), `NUMERIC` (Decimal), `BOOLEAN` (Bool), `JSONB` (lists, records,
`Data`) and `TEXT` (the rest).

- A table that does not exist is created with a column per written field.
- `mode = "create"` refuses an existing table; `append` inserts after the
  rows held; `replace` (the default) empties the table with `TRUNCATE` and
  keeps its definition, indexes and grants.
- Each value is sent as text and cast by the server to the column's type, so
  a `Decimal` keeps its digits.
- `target` writes every row in one transaction. `sink` applies the mode when
  it opens the table and commits each row as it arrives.

### 5. Reading a topic

- A read connects when the stream is first traversed, lists the topic's
  partitions, and starts each at the earliest or latest offset `from` names.
- Partitions are fetched in turn; each message's value is one JSON document.
  A message without a value is skipped.
- With `idle_ms`, the stream ends once no message has arrived for that long;
  without it the stream does not end.
- A topic is always read through its boundary, so what a write produced
  through the boundary, a `Decimal` as JSON text among it, reads back as it
  was written.
- A connection or a request that keeps failing for 10 seconds is an
  evaluation failure naming the source.

### 6. Writing a topic

- Each value is one message: its document through the boundary where the
  element is a declared record, and its JSON otherwise.
- Every message goes to the topic's first partition, so the messages one
  program writes keep their order. They are sent in batches of 100,
  uncompressed.
- A topic that does not exist is an evaluation failure; creating topics is
  the cluster's business.

## Security Considerations

- Secrets are read from the environment where a source is used, never from
  `hql.toml`, and a failure message never repeats a connection string.
- A document being rendered reaches neither system: reads and writes are
  refused under the document policy.
- Table and column names are quoted as identifiers in every statement; values
  are bound as parameters.

## Reference Implementation

- `src/endpoints.rs` for the two kinds, `Brokers` and `secret`.
- `src/postgres.rs` for columns, rows and the writers, on the synchronous
  `postgres` client.
- `src/kafka.rs` for the reader and the producer, on `rskafka` with gzip and
  snappy only, each connection on its own one-thread `tokio` runtime.
- `src/extensions/io.rs` for the types, `sink("id")` and the boundary;
  `src/boundary.rs` for the `postgres` type table.

## Test Plan

`tests/topics_and_tables.rs` checks the configuration errors, the types, the
refusals and the failure when a secret's variable is unset, with no network.
The round trips run against PostgreSQL 17 and Kafka 3.9 in Docker Compose, in
the `hql-demo` repository: a JSON file to a topic and back, a JSON file to a
table and back, and a table to a topic to a second table.

```bash
cargo test --test topics_and_tables
../hql-demo/run-all.sh
```

## Changelog

- 2026-11-12: accepted and implemented
