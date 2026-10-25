<!-- hql reference: std/core.Ord -->
# `Ord`

In [`std/core`](../index.md).

```hql
trait Ord
```

Order: `a < b` and the other comparisons over a type implementing `Ord`
call its `compare`. A type implementing `Ord` is `Orderable`: `sort`,
`min` and `max` take it, and its `compare` gives the order.

## Members

```hql
fn compare(self, other: Self) : Ordering
```
<!-- hql reference: end -->
