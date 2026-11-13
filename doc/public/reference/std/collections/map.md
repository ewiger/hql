<!-- hql reference: std/collections.map -->
# `map`

In [`std/collections`](index.md).

```hql
elementwise fn map<F<_> : Iterable<_>, T, R>(self: F<T>, by: T -> R) : F<R>
elementwise fn map<T, R>(self: Option<T>, by: T -> R) : Option<R>
elementwise fn map<T, R, E>(self: Result<T, E>, by: T -> R) : Result<R, E>
```

Each element through `by`, in a collection of the input's constructor:
`[1, 2, 3] | map(x => x * 10)` is `[10, 20, 30]`. A `Set` maps to its
image, which may hold fewer elements than the input. Over an `Option`,
`by` applies to a present value and `none` stays `none`; over a
`Result`, `by` applies to the success value and a failure is unchanged.
<!-- hql reference: end -->
