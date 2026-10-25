<!-- hql reference: std/math -->
# `std/math`

```hql
module math
```

Numbers, in the prelude. `abs`, `pow` and `sign` take an `Int` or a
`Float`, and `abs` and `sign` a `Decimal` too; `round` takes a `Float` or
a `Decimal`, and the rest take a `Float`. `float`, `int` and `decimal`
convert between the three, since no number widens on its own except an
`Int` beside a `Decimal`.

- [`abs`](abs.md) — `self` without its sign, at its own type: `abs(-3)` is `3`, and `abs(-1.50)` is `1.50`.
- [`pow`](pow.md) — `self` raised to `exponent`, both of one type: `pow(2, 10)` is `1024`.
- [`sign`](sign.md) — `-1`, `0` or `1` by the sign of `self`, as an `Int`: `sign(-0.5)` is `-1`.
- [`float`](float.md) — `self` as a `Float`, the `Float` nearest it: `float(1.10)` is `1.1`.
- [`int`](int.md) — The integer part of `self`, as an `Int`, the fraction dropped toward zero: `int(-2.9f)` is `-2`.
- [`decimal`](decimal.md) — `self` as a `Decimal`: an `Int` exactly, and a `Float` by the shortest digits that read back as it: `decimal(0.1f)` is `0.1`.
- [`gcd`](gcd.md) — The greatest common divisor of `self` and `other`, at least 0: `gcd(12, 18)` is `6`, and `gcd(0, 0)` is `0`.
- [`sqrt`](sqrt.md) — The square root of `self`: `sqrt(2.0f)` is `1.4142135623730951`.
- [`floor`](floor.md) — The greatest whole number not above `self`, as a `Float`: `floor(-1.5f)` is `-2.0`.
- [`ceil`](ceil.md) — The least whole number not below `self`, as a `Float`: `ceil(-1.5f)` is `-1.0`.
- [`round`](round.md) — The whole number nearest `self`, as a `Float`; a value halfway between two rounds away from zero: `2.5f | round` is `3.0`.
- [`exp`](exp.md) — e raised to `self`: `exp(1.0f)` is `2.718281828459045`.
- [`ln`](ln.md) — The natural logarithm of `self`: `ln(1.0f)` is `0.0`.
- [`log10`](log10.md) — The base-10 logarithm of `self`: `log10(1000.0f)` is `3.0`.
- [`sin`](sin.md) — The sine of `self`, an angle in radians: `sin(0.0f)` is `0.0`.
- [`cos`](cos.md) — The cosine of `self`, an angle in radians: `cos(0.0f)` is `1.0`.
- [`tan`](tan.md) — The tangent of `self`, an angle in radians: `tan(0.0f)` is `0.0`.
<!-- hql reference: end -->
