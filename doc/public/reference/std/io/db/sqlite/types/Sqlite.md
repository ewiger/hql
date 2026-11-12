<!-- hql reference: std/io/db/sqlite.Sqlite -->
# `Sqlite`

In [`std/io/db/sqlite`](../index.md).

```hql
type Sqlite : {Direct, RowTransport, Replayable, Writable}
```

One table of an SQLite database file. It is on this machine and carries
rows of named columns, so it takes no format. A table ends and reads the
same rows each time in one evaluation, so it is `Replayable`: read, it
is a `Seq`, a row per element.
<!-- hql reference: end -->
