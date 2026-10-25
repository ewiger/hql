<!-- hql reference: std/seq.intersection -->
# `intersection`

In [`std/seq`](index.md).

```hql
fn intersection<S : Set<T>, T>(self: S, other: Set<T>) : S
```

The members of both `self` and `other`, at the type of `self`:
`{1, 2, 3} | intersection({2, 3, 4})` is `{2, 3}`. The union is the
core's `union`.
<!-- hql reference: end -->
