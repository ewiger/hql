<!-- hql reference: std/collections.cross_join -->
# `cross_join`

In [`std/collections`](index.md).

```hql
fn cross_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, into: (T, U) -> R) : Seq<R>
```

Every pair of an element of the input and an element of `other`, built
by `into`, in order, with no condition:
`[1, 2] | cross_join(["a", "b"], into = (x, y) => [x, y])` is
`[[1, "a"], [1, "b"], [2, "a"], [2, "b"]]`.
<!-- hql reference: end -->
