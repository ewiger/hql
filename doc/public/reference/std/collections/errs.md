<!-- hql reference: std/collections.errs -->
# `errs`

In [`std/collections`](index.md).

```hql
elementwise fn errs<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<E>
```

The error values of a collection of results, in an `F<E>`; the
successes are dropped: `[ok(1), err("x"), ok(3)] | errs` is `["x"]`.

```hql
elementwise fn errs<T, E>(self: Stream<Result<T, E>>) : Stream<E>
```

The error values of a stream of results, as a stream; the successes are
dropped.
<!-- hql reference: end -->
