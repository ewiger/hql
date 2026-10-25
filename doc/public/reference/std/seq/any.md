<!-- hql reference: std/seq.any -->
# `any`

In [`std/seq`](index.md).

```hql
reduction fn any(self: Collection<T>, by: T -> Bool) : Bool
```

Whether `by` holds of some value of `self`; `false` over an empty
collection: `[1, 2, 3] | any(x => x > 2)` is `true`.
<!-- hql reference: end -->
