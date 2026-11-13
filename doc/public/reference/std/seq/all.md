<!-- hql reference: std/seq.all -->
# `all`

In [`std/seq`](index.md).

```hql
reduction fn all(self: Collection<T>, by: T -> Bool) : Bool
```

Whether `by` holds of every value of `self`; `true` over an empty
collection: `[] | all(x => x > 2)` is `true`.
<!-- hql reference: end -->
