<!-- hql reference: std/collections.somes -->
# `somes`

In [`std/collections`](index.md).

```hql
elementwise fn somes<F<_> : Collection<_>, T>(self: F<Option<T>>) : F<T>
```

The present values of a collection of options, in an `F<T>`; the absent
ones are dropped: `[some(1), none, some(3)] | somes` is `[1, 3]`.
<!-- hql reference: end -->
