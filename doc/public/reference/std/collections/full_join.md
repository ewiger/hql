<!-- hql reference: std/collections.full_join -->
# `full_join`

In [`std/collections`](index.md).

```hql
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, K, R>(self: Seq<T>, other: Seq<U>, *, left: T -> K, right: U -> K, into: (Option<T>, Option<U>) -> R) : Seq<R>
fn full_join<T, U, R>(self: Seq<T>, other: Seq<U>, *, on: (T, U) -> Bool, into: (Option<T>, Option<U>) -> R) : Seq<R>
```

A `join` that keeps every element of both sides, with `none` for the
missing side; the unmatched elements of `other` come after the matched
results.
<!-- hql reference: end -->
