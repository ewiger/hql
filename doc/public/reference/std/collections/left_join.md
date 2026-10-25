<!-- hql reference: std/collections.left_join -->
# `left_join`

In [`std/collections`](index.md).

```hql
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (T, Option<U>) -> R) : Seq<R>
fn left_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (T, Option<U>) -> R) : Seq<R>
```

A `join` that keeps every element of the input: one with no match in
`other` is built by `into` with `none` on the right.
<!-- hql reference: end -->
