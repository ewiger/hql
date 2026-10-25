<!-- hql reference: std/core.Partition -->
# `Partition`

In [`std/core`](../index.md).

```hql
abstract type Partition<I, O> : Operation<I, O>
```

An `Operation` that regroups its input by a key, each group running
independently and completing in any order: `group` is one.
<!-- hql reference: end -->
