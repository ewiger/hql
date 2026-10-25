<!-- hql reference: std/collections.average -->
# `average`

In [`std/collections`](index.md).

```hql
reduction fn average(self: Collection<Int>) : Option<Float>
reduction fn average(self: Collection<Float>) : Option<Float>
reduction fn average(self: Collection<Decimal>) : Option<Decimal>
```

The mean of the occurrences, or `none` over `[]`:
`[1, 2] | average` is `1.5`. `Int`s and `Float`s average to a `Float`;
a `Decimal` mean is the sum divided by the count, as `Decimal` `/`
divides.
<!-- hql reference: end -->
