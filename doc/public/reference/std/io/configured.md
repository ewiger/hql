<!-- hql reference: std/io.configured -->
# `configured`

In [`std/io`](index.md).

```hql
fn configured(id: String) : Transport
```

The endpoint configured under the literal `id` in hql.toml, as the
transport its kind states, with its schema boundary: a `postgres`
endpoint is a `Postgres`, a `csv` endpoint a `File`. Constructing it
opens nothing, and it needs no import: the configuration names the
kind. `source(configured("id"))` reads what `source("id")` reads, and
`target(xs, configured("id"))` writes what `target(xs, "id")` writes;
the value is what a statement over the endpoint itself takes
(HQL-0051). A `vault` endpoint has no transport and is refused.
<!-- hql reference: end -->
