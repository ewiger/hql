# 0062 — NATS subjects and lakehouse tables as configured endpoints

Status: doing
Branch: `feat/nats-lakehouse`
Related: [HQL-0049](../proposals/HQL-0049/README.md),
[HQL-0046](../proposals/HQL-0046/README.md),
[HQL-0035](../proposals/HQL-0035/README.md)

Two kinds join `[endpoint.<id>]`, one on each side of the line `source` and
`tap` draw. `kind = "nats"` is one subject of a JetStream stream, a log that
does not end. `kind = "spark"` is one table behind a Spark SQL endpoint, a
Databricks SQL warehouse or a Spark Thrift Server, which ends.

## Work

1. `src/nats.rs` reads a stream's subject through an ordered consumer and
   publishes with acknowledgement; `src/spark.rs` speaks HiveServer2's
   Thrift protocol over HTTP.
2. `src/endpoints.rs` reads the two kinds; `src/extensions/io.rs` types
   them and wires `source`, `tap`, `target` and `sink`; `src/boundary.rs`
   holds the `spark` type table.
3. A list or a record field written through the boundary of a format that
   nests is written as a tree, not as its text.
4. `tests/subjects_and_lakehouse_tables.rs` covers the configuration, the
   types and the refusals offline.
5. The `hql-demo` repository runs both against real servers in Docker
   Compose: `nats/`, `lakehouse/` and `nats-lakehouse/`.
6. HQL-0049, the changelog, `doc/stack.md` and the book's I/O pages state
   the two kinds.

## Done when

- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
- `../hql-demo/run-all.sh` passes.
