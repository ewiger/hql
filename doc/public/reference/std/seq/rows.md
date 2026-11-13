<!-- hql reference: std/seq.rows -->
# `rows`

In [`std/seq`](index.md).

```hql
fn rows(self: Window<T>, before: Int, after: Int) : List<T>
```

The rows from `before` positions before the row to `after` positions
after it, clipped to the partition. Either below 0 is an evaluation
error.
<!-- hql reference: end -->
