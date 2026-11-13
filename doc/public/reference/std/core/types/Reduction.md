<!-- hql reference: std/core.Reduction -->
# `Reduction`

In [`std/core`](../index.md).

```hql
abstract type Reduction<I, O> : Operation<I, O>
```

An `Operation` that collapses its whole input to one value through a
combiner the runtime holds, so each split may be reduced and the results
combined: `size` combines by addition, `contains` by disjunction.
<!-- hql reference: end -->
