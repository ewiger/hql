<!-- hql reference: std/collections.join -->
# `join`

In [`std/collections`](index.md).

```hql
fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>
fn join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, U) -> R) : Seq<R>
fn join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, U) -> R) : Seq<R>
```

The pairs of an element of the input and an element of `other` that
match, each built by `into`, ordered by the input element's position,
then `other`'s. A pair matches when `left(x) == right(y)`, when `on`
holds for it, or both; a join states at least one condition.
<!-- hql reference: end -->
