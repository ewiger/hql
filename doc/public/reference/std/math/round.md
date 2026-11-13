<!-- hql reference: std/math.round -->
# `round`

In [`std/math`](index.md).

```hql
fn round(self: Float) : Float
```

The whole number nearest `self`, as a `Float`; a value halfway between
two rounds away from zero: `2.5f | round` is `3.0`.

```hql
fn round(self: Decimal) : Decimal
fn round(self: Decimal, places: Int) : Decimal
```

`self` rounded half to even to `places` digits after the point, and
kept at that scale: `round(2.345, 2)` is `2.34`, and `round(x)` is
`round(x, 0)`. See [Division and rounding](../../../learn/02-data-types/decimal.md#division-and-rounding).
<!-- hql reference: end -->
