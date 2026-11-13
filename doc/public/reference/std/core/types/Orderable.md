<!-- hql reference: std/core.Orderable -->
# `Orderable`

In [`std/core`](../index.md).

```hql
abstract type Orderable
```

A type whose values have a total order: of any two values of the type,
one comes first or the two are equal. `compare`, `<`, `sort`, `min` and
`max` take it, and `SortedMap` keys are of it.
Order is a property of the element, not of a collection holding it.
<!-- hql reference: end -->
