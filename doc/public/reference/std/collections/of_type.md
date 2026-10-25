<!-- hql reference: std/collections.of_type -->
# `of_type`

In [`std/collections`](index.md).

```hql
elementwise fn of_type<F<_> : Collection<_>, T, U>(self: F<T>, type: Type<U>) : F<U>
```

The elements whose runtime type is `type` or one of its subtypes, each
at that type, in an `F<U>`: `filter(x => x | is(U))`.
`[1, "a", 2] | of_type(Int)` is `[1, 2]`.
<!-- hql reference: end -->
