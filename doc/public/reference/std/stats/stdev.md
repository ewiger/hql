<!-- hql reference: std/stats.stdev -->
# `stdev`

In [`std/stats`](index.md).

```hql
reduction fn stdev(self: Collection<Int>) : Option<Float>
reduction fn stdev(self: Collection<Float>) : Option<Float>
reduction fn stdev(self: Collection<Decimal>) : Option<Decimal>
```

The sample standard deviation, the square root of `variance`, or `none`
over fewer than two values. A `Decimal` result is the square root
rounded half to even to 100 significant digits.
<!-- hql reference: end -->
