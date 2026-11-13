<!-- hql reference: std/io/db/spark -->
# `std/io/db/spark`

```hql
module io.db.spark
```

Spark SQL: one table behind a Spark SQL endpoint, a Databricks SQL
warehouse or a Spark Thrift Server, as a transport read by `source` and
`tap` and written by `target` like any other endpoint (module `io`). The
`spark` kind of a configured endpoint is this transport under an id,
with a schema boundary. A program imports the module:
`import io.db.spark`.

- [`Spark`](types/Spark.md) — One table behind a Spark SQL endpoint.
- [`spark`](spark.md) — The table `table`, qualified by catalog and schema where it holds dots, behind the Spark SQL endpoint at `url` or at the address the environment variable `url_env` holds, one of the two, as a `Spark` transport; its parameters are the keys of the `spark` kind, each written by name.
<!-- hql reference: end -->
