<!-- hql reference: std/math.pow -->
# `pow`

In [`std/math`](index.md).

```hql
fn pow(self: Int, exponent: Int, /) : Int
fn pow(self: Float, exponent: Float, /) : Float
```

`self` raised to `exponent`, both of one type: `pow(2, 10)` is `1024`.
Over `Int`s an `exponent` below 0 is an evaluation error, and so is a
result past the range of `Int`.
<!-- hql reference: end -->
