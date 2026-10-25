<!-- hql reference: std/seq.difference -->
# `difference`

In [`std/seq`](index.md).

```hql
fn difference<S : Set<T>, T>(self: S, other: Set<T>) : S
```

The members of `self` that `other` does not hold, at the type of `self`:
`{1, 2, 3} | difference({2, 3, 4})` is `{1}`.
<!-- hql reference: end -->
