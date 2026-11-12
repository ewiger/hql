# HQL-0049: NATS subjects and lakehouse tables as configured endpoints

**Status**: accepted, implemented
**Created**: 2026-11-12
**Source**: [HQL-0046](../HQL-0046/README.md) (Kafka topics and PostgreSQL
tables), [HQL-0035](../HQL-0035/README.md) (the schema boundary and write
modes), [HQL-0021](../HQL-0021/README.md) (endpoints, `source` and `tap`),
`src/nats.rs`, `src/spark.rs`, `src/endpoints.rs`, `src/extensions/io.rs`,
`src/boundary.rs`

## Abstract

Two kinds join `[endpoint.<id>]` in `hql.toml`. `kind = "nats"` is one
subject of a NATS JetStream stream: a log that does not end, read by
`tap("id")` as a `Stream`, one JSON document per message, and written by
`sink("id")` or `target("id")`, one message per value. `kind = "spark"` is
one table behind a Spark SQL endpoint, which is a Databricks SQL warehouse
or cluster, or an open-source Spark Thrift Server: a finite source, read by
`source("id")` as a `Seq` through the boundary, and written by `target("id")`
or `sink("id")`. Each behaves as the kind of
[HQL-0046](../HQL-0046/README.md) on its side of that line does.

## Motivation

- A message bus and a lakehouse are the two systems a data platform has
  besides a broker and a relational database. A pipeline file reaches them
  by id, with its types known offline.
- One protocol reaches a lakehouse wherever it runs. A Databricks SQL
  warehouse and a Spark Thrift Server both answer HiveServer2's Thrift
  protocol over HTTP, so one kind, one configuration and one program run
  against either.

## Non-goals

- Core NATS subjects that no stream holds: a message nothing stores cannot
  be read from the earliest one.
- Durable consumers, acknowledgement of messages read, and key-value or
  object stores.
- NATS credential files and NKeys; TLS options beyond what the address
  states.
- Nested columns: `ARRAY`, `MAP`, `STRUCT` and `VARIANT`.
- Databricks OAuth, and Arrow or cloud-fetched results.
- Running a pipeline on Spark: the kind reaches one table, and evaluation
  stays in `hql`.

## Specification

### 1. Configuration

Each kind takes its own keys besides `kind`, `element`, `fields`, `mode` and
`strictness`.

- Both kinds MUST name their address once, by `url` or by `url_env`, the
  environment variable that holds it.
- A `url` that holds `@` refuses the vault, naming `url_env`: an address
  with credentials belongs in the environment.
- `nats` MUST name `stream`, the JetStream stream that holds the messages,
  and `subject`, the subject read within it and published to. It MAY name
  `from`, `earliest` (the default) or `latest`, and `idle_ms`, a count above
  0.
- `spark` MUST name `table`, qualified by catalog and schema where it holds
  dots (`main.sales.orders`). It MAY name `order`, the columns rows are read
  in, and `token_env`, the environment variable that holds an access token.
- `token` is a secret key: written in the configuration it refuses the
  vault, naming `token_env`.
- A `nats` or `spark` endpoint takes no `*_env` key but its own.

```toml
[endpoint.events]
kind = "nats"
url_env = "NATS_URL"
stream = "EVENTS"
subject = "events.orders"
idle_ms = 2000

[endpoint.orders]
kind = "spark"
url_env = "LAKEHOUSE_URL"
token_env = "LAKEHOUSE_TOKEN"
table = "main.sales.orders"
order = ["id"]

[endpoint.orders.fields.total]
type = "DECIMAL(12,2)"
```

### 2. Types

The kind states the type, read from the configuration at check time;
checking opens no connection.

- `tap("id")` of a `nats` endpoint is a `Stream` of the element.
  `source("id")` of it MUST be a type error: `` `events` is a `nats`
  endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq` ``.
- `source("id")` of a `spark` endpoint is a `Seq` of the element;
  `tap("id")` gives its rows as a `Stream`.
- `sink(xs, "id")` and `target(xs, "id")` of a `nats` endpoint take values of
  any type; of a `spark` endpoint, rows of a declared record.

### 3. Reading a subject

- A read connects when the stream is first traversed and creates an ordered
  consumer on `stream`, filtered to `subject`. The consumer is ephemeral:
  the server keeps no position once the read ends.
- `subject` MAY hold the wildcards `*` and `>`, and then reads every subject
  of the stream that matches.
- Messages arrive in the stream's order, from the earliest the stream keeps
  or, under `from = "latest"`, from the first published after the read
  begins. Each message's payload is one JSON document.
- With `idle_ms`, the stream ends once no message has arrived for that
  long; without it the stream does not end.
- A subject is always read through its boundary, as a topic is.

### 4. Writing a subject

- Each value is one message: its document through the boundary where the
  element is a declared record, and its JSON otherwise.
- A message is written once a stream acknowledges that it stored it.
  Acknowledgements are awaited every 100 messages and at the end. A subject
  no stream holds is an evaluation failure.
- A `subject` that holds a wildcard MUST be refused before any connection: a
  message is published to one subject.
- A list or a record field of a message is written as the tree it holds.
  This holds for every format that nests, a topic and a JSON file among
  them.

### 5. Reading a lakehouse table

- A read opens a session, runs one `SELECT` of the columns the boundary
  reads, ordered by `order`, and fetches the result column by column.
- The table's columns are read before any row and checked against the
  boundary; strictness applies to an unmapped column there.
- `TINYINT`, `SMALLINT`, `INT` and `BIGINT` read as an integer, `FLOAT` and
  `DOUBLE` as a float, `DECIMAL` as a decimal with its digits, `BOOLEAN` as
  a boolean, and every other type as its text. `NULL` is absent by default.

### 6. Writing a lakehouse table

The boundary's types are `BIGINT` (Int), `DOUBLE` (Float), `DECIMAL(38,18)`
(Decimal), `BOOLEAN` (Bool) and `STRING` (the rest). A `DECIMAL` states its
precision and scale, so a field's `type` SHOULD name the one its values
have. A list or a record field has no column type and is a boundary error.

- A table that does not exist is created with a column per written field,
  in the endpoint's default format, which is Delta Lake on Databricks.
- `mode = "create"` refuses an existing table; `append` inserts after the
  rows held; `replace` (the default) puts the rows in place of the ones
  held and keeps the table's definition.
- The protocol binds no parameters. Each value is written as a literal the
  server casts to the column's type, so a `Decimal` keeps its digits. Text
  of letters, digits and plain punctuation is written between quotes; any
  other text is written as the hexadecimal digits of its bytes, so no
  character of a value is read as SQL.
- Rows are written in statements of up to 500. A statement is one commit:
  `target` of up to 500 rows is atomic, and under `replace` its first
  statement overwrites the rows held in the same commit.
- `sink` applies the mode when it opens the table, emptying it under
  `replace`, and writes a statement each 500 rows and at the end.

### 7. The protocol

- Each call is one HTTP `POST` of a Thrift binary message to the address:
  `OpenSession`, `ExecuteStatement`, `GetOperationStatus`,
  `GetResultSetMetadata`, `FetchResults`, `CloseOperation`, `CloseSession`.
- A statement runs asynchronously and its state is polled, so no HTTP
  request is held open while it runs.
- With `token_env`, the token is sent as the password of the user `token`,
  which is how a Databricks warehouse reads a personal access token.
  Without it, a fixed user is sent, which a Thrift Server without
  authentication accepts.
- A connection that is not made within 10 seconds is an evaluation failure
  naming the endpoint.

## Security Considerations

- An address with credentials and a token are read from the environment
  where an endpoint is used, never from `hql.toml`. A failure message
  repeats neither.
- A document being rendered reaches neither system.
- Table and column names are quoted as identifiers. A value never reaches
  the server as SQL text unless every character of it is a letter, a digit
  or one of `` -_.:,+/@`` and a space; otherwise it is sent as hexadecimal
  digits and decoded by the server.

## Reference Implementation

- `src/endpoints.rs` for the two kinds, `Address` and `Start`.
- `src/nats.rs` for the reader and the publisher, on `async-nats` with its
  `jetstream` feature, each connection on its own one-thread `tokio`
  runtime.
- `src/spark.rs` for the Thrift codec, the session, columns, rows and the
  writers, on `ureq`.
- `src/extensions/io.rs` for the types and the four operations;
  `src/boundary.rs` for the `spark` type table.

## Test Plan

`tests/subjects_and_lakehouse_tables.rs` checks the configuration errors,
the types, the refusals and the failure when a variable is unset, with no
network; `src/spark.rs` tests the codec, the literals and the statements.
The round trips run in Docker Compose, in the `hql-demo` repository, against
NATS 2.11 with JetStream and against Spark 3.5 with Delta Lake 3.3 on MinIO:
a JSON file to a subject and back, a JSON file to a Delta table and back,
and a subject routed into a Delta table and a dead-letter subject, then
aggregated into a second table.

No run has been made against a Databricks workspace.

```bash
cargo test --test subjects_and_lakehouse_tables
cargo test --lib spark
../hql-demo/run-all.sh
```

## Changelog

- 2026-11-12: accepted and implemented
