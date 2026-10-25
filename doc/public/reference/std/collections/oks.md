<!-- hql reference: std/collections.oks -->
# `oks`

In [`std/collections`](index.md).

```hql
elementwise fn oks<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<T>
```

The success values of a collection of results, in an `F<T>`; the
failures are dropped: `[ok(1), err("x"), ok(3)] | oks` is `[1, 3]`.

```hql
elementwise fn oks<T, E>(self: Stream<Result<T, E>>) : Stream<T>
```

The success values of a stream of results, as a stream; the failures
are dropped.
<!-- hql reference: end -->
