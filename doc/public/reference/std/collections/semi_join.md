<!-- hql reference: std/collections.semi_join -->
# `semi_join`

In [`std/collections`](index.md).

```hql
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn semi_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn semi_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>
```

The elements of the input that match some element of `other`, by keys,
a predicate `on`, or both; each is kept at most once, in its position.
<!-- hql reference: end -->
