<!-- hql reference: std/format.Table -->
# `Table`

In [`std/format`](../index.md).

```hql
type Table<R>
```

Rows of type `R` under ordered columns: the shape every format's encoder
writes as a table. `R` is a record type or `Data`; `Table` written
without it is `Table<Data>`.

```hql
fn Table<T>(rows: Collection<T>, columns: List<String> = []) : Table<T>
```

A `Table` of `rows`, its columns named by `columns` in the order they are
written; with `columns` empty, the columns are the first row's fields in
the order the row holds them. Over a declared record, a column naming no
field is a type error; over `Data`, a missing field is an empty cell.
`Table([{a: 1, b: 2}], ["b"]) | csv` is the header `b` and the row `2`.
<!-- hql reference: end -->
