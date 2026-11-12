<!-- hql reference: std/io/db/sqlite.sqlite -->
# `sqlite`

In [`std/io/db/sqlite`](index.md).

```hql
fn sqlite(path: String, table: String, order: List<String> = []) : Sqlite
```

The table `table` of the database file at `path`, as a `Sqlite`
transport; its parameters are the keys of the `sqlite` kind. A relative
path resolves as a file's does. Rows are read in the order of the
columns `order` names, and in `rowid` order without them. Constructing
it opens nothing.

A constructed table has no schema boundary: a row is read as `Data`, or
as the type an annotation names, and a declared record is written a
column per field, the table replaced. Its columns, `mode` and
`strictness` are configured on a `sqlite` endpoint.
<!-- hql reference: end -->
