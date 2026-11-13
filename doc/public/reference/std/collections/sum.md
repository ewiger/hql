<!-- hql reference: std/collections.sum -->
# `sum`

In [`std/collections`](index.md).

```hql
reduction fn sum(self: Collection<Int>) : Int
reduction fn sum(self: Collection<Float>) : Float
reduction fn sum(self: Collection<Decimal>) : Decimal
```

The sum of the occurrences, `reduce(0, +)`; `[] | sum` is `0`.
<!-- hql reference: end -->
