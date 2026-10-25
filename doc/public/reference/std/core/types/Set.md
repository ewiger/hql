<!-- hql reference: std/core.Set -->
# `Set`

In [`std/core`](../index.md).

```hql
type Set<T> : Collection<T>
```

Materialized distinct values under HQL equality: `count(s, x) <= 1`.
Traversal order is not a semantic guarantee.

```hql
fn Set<T>(xs: Collection<T>) : Set<T>
```

The `Set` of the distinct elements of `xs`.
<!-- hql reference: end -->
