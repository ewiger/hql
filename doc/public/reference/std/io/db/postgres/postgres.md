<!-- hql reference: std/io/db/postgres.postgres -->
# `postgres`

In [`std/io/db/postgres`](index.md).

```hql
fn postgres(*, dsn_env: String, table: String, order: List<String> = []) : Postgres
```

The table `table` of the database whose connection string the
environment variable `dsn_env` holds, as a `Postgres` transport; its
parameters are the keys of the `postgres` kind, each written by name. A
connection string may hold a password, so a program names the variable
and never the string.
`table` is schema-qualified where it holds a dot, `audit.events`. Rows
are read in the order of the columns `order` names, and as the table is
scanned without them. Constructing it opens no connection.

A constructed table has no schema boundary: a row is read as `Data`, or
as the type an annotation names, and a declared record is written a
column per field, the table's rows replaced. Its columns, `mode` and
`strictness` are configured on a `postgres` endpoint.
<!-- hql reference: end -->
