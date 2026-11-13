<!-- hql reference: std/stats.mode -->
# `mode`

In [`std/stats`](index.md).

```hql
fn mode<T : Orderable>(self: Collection<T>) : Option<T>
```

The most frequent value, and among equally frequent values the least by
the type's order, or `none` over an empty collection:
`[3, 1, 3, 1, 2] | mode` is `1`.
<!-- hql reference: end -->
