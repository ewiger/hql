<!-- hql reference: std/math.abs -->
# `abs`

In [`std/math`](index.md).

```hql
fn abs(self: Int) : Int
fn abs(self: Float) : Float
fn abs(self: Decimal) : Decimal
```

`self` without its sign, at its own type: `abs(-3)` is `3`, and
`abs(-1.50)` is `1.50`.
<!-- hql reference: end -->
