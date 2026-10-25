<!-- hql reference: std/seq.find -->
# `find`

In [`std/seq`](index.md).

```hql
fn find(self, by: T -> Bool) : Option<T>
```

The first element of `self` that `by` accepts, or `none`:
`[1, 2, 3, 4] | find(x => x > 2)` is `3`.
<!-- hql reference: end -->
