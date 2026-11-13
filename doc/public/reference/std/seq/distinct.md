<!-- hql reference: std/seq.distinct -->
# `distinct`

In [`std/seq`](index.md).

```hql
fn distinct<S : Seq<T>>(self: S) : S
```

Each value of `self` once, at its first position, at the type of `self`:
`[1, 2, 1, 3, 2] | distinct` is `[1, 2, 3]`.
<!-- hql reference: end -->
