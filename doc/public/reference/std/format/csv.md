<!-- hql reference: std/format.csv -->
# `csv`

In [`std/format`](index.md).

```hql
fn csv<T>(x: Collection<T>) : String
fn csv<T>(x: Stream<T>) : Stream<String>
fn csv<T>(x: T) : String
```

`x` encoded as `Csv`: `x | csv` is `format(x, Csv)`.
<!-- hql reference: end -->
