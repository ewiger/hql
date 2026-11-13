<!-- hql reference: std/collections.right_join -->
# `right_join`

In [`std/collections`](index.md).

```hql
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, U) -> R) : Seq<R>
fn right_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, U) -> R) : Seq<R>
```

A `join` that keeps every element of `other`: one with no match in the
input is built by `into` with `none` on the left.
<!-- hql reference: end -->
