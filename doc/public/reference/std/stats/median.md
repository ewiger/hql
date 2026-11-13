<!-- hql reference: std/stats.median -->
# `median`

In [`std/stats`](index.md).

```hql
fn median(self: Collection<Int>) : Option<Float>
fn median(self: Collection<Float>) : Option<Float>
fn median(self: Collection<Decimal>) : Option<Decimal>
```

The middle value of the collection sorted ascending, the mean of the two
middle values where the count is even; `quantile(0.5)`. `none` over an
empty collection: `[3, 1, 4, 2] | median` is `2.5`.
<!-- hql reference: end -->
