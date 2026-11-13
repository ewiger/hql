<!-- hql reference: std/io/db/postgres -->
# `std/io/db/postgres`

```hql
module io.db.postgres
```

PostgreSQL: one table of a database, as a transport read by `source` and
`tap` and written by `target` like any other endpoint (module `io`). The
`postgres` kind of a configured endpoint is this transport under an id,
with a schema boundary. A program imports the module:
`import io.db.postgres`.

- [`Postgres`](types/Postgres.md) — One table of a PostgreSQL database.
- [`postgres`](postgres.md) — The table `table` of the database whose connection string the environment variable `dsn_env` holds, as a `Postgres` transport; its parameters are the keys of the `postgres` kind.
<!-- hql reference: end -->
