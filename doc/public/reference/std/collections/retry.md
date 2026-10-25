<!-- hql reference: std/collections.retry -->
# `retry`

In [`std/collections`](index.md).

```hql
elementwise fn retry<F<_> : Iterable<_>, T, U, E>(self: F<T>, operation: T -> Result<U, E>, attempts: Int) : F<Result<U, E>>
elementwise fn retry<F<_> : Iterable<_>, T, U, E>(self: F<T>, operation: T -> Result<U, E>, attempts: Int, when: E -> Bool) : F<Result<U, E>>
```

Each element through the fallible `operation`, invoked again while it
fails with an error `when` accepts (every error, without `when`) and
fewer than `attempts` invocations have run. The result is each
element's last result: its success, or its last failure.
<!-- hql reference: end -->
