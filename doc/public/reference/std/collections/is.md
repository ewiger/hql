<!-- hql reference: std/collections.is -->
# `is`

In [`std/collections`](index.md).

```hql
fn is<T, U>(self: T, type: Type<U>) : Bool
```

`true` when the runtime type of `self` is `type` or one of its subtypes.
The checker rejects a test no `T` can pass, and warns about one every
`T` passes. Used as a `filter` predicate, it narrows the result.
<!-- hql reference: end -->
