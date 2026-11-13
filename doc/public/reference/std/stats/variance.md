<!-- hql reference: std/stats.variance -->
# `variance`

In [`std/stats`](index.md).

```hql
reduction fn variance(self: Collection<Int>) : Option<Float>
reduction fn variance(self: Collection<Float>) : Option<Float>
reduction fn variance(self: Collection<Decimal>) : Option<Decimal>
```

The sample variance, the sum of squared deviations from the mean divided
by `n - 1`, or `none` over fewer than two values:
`[1, 2, 3, 4] | stats.variance` is `1.6666666666666667`.
<!-- hql reference: end -->
