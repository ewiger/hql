<!-- hql reference: std/stats.correlation -->
# `correlation`

In [`std/stats`](index.md).

```hql
reduction fn correlation(self: Collection<Pair<Int, Int>>) : Option<Float>
reduction fn correlation(self: Collection<Pair<Float, Float>>) : Option<Float>
reduction fn correlation(self: Collection<Pair<Decimal, Decimal>>) : Option<Float>
```

Pearson's correlation coefficient of the pairs' `first` and `second`
values, the covariance divided by the product of the two standard
deviations, or `none` over fewer than two pairs or where either side has
zero variance.
<!-- hql reference: end -->
