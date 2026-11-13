<!-- hql reference: std/stats.pstdev -->
# `pstdev`

In [`std/stats`](index.md).

```hql
reduction fn pstdev(self: Collection<Int>) : Option<Float>
reduction fn pstdev(self: Collection<Float>) : Option<Float>
reduction fn pstdev(self: Collection<Decimal>) : Option<Decimal>
```

The population standard deviation, the square root of `pvariance`, or
`none` over an empty collection. A `Decimal` result is the square root
rounded half to even to 100 significant digits.
<!-- hql reference: end -->
