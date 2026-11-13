<!-- hql reference: std/core.Barrier -->
# `Barrier`

In [`std/core`](../index.md).

```hql
abstract type Barrier<I, O> : Operation<I, O>
```

An `Operation` that needs its whole input before any output, so it is
not split: `sort`, `take` and `reduce` are barriers.
<!-- hql reference: end -->
