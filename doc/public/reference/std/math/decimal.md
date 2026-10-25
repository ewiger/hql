<!-- hql reference: std/math.decimal -->
# `decimal`

In [`std/math`](index.md).

```hql
fn decimal(self: Int) : Decimal
fn decimal(self: Float) : Decimal
```

`self` as a `Decimal`: an `Int` exactly, and a `Float` by the shortest
digits that read back as it: `decimal(0.1f)` is `0.1`.
<!-- hql reference: end -->
