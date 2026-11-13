<!-- hql reference: std/collections.anti_join -->
# `anti_join`

In [`std/collections`](index.md).

```hql
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool) : Seq<T>
fn anti_join<T, U, K>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K) : Seq<T>
fn anti_join<T, U>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool) : Seq<T>
```

The elements of the input that match no element of `other`, by keys, a
predicate `on`, or both, in their positions.
<!-- hql reference: end -->
