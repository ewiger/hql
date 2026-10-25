<!-- hql reference: std/seq.join -->
# `join`

In [`std/seq`](index.md).

```hql
fn join<U>(self: Collection<T>, other: Collection<U>, by: (T, U) -> Bool) : List<Pair<T, U>>
```

Every pair of a value of `self` and a value of `other` that `by` relates,
in the order of `self`, then of `other`:
`[1, 2] | join([1, 2, 3], (a, b) => a < b)` gives the pairs `(1, 2)`,
`(1, 3)` and `(2, 3)`.
<!-- hql reference: end -->
