# Decimal

A `Decimal` is an exact decimal number: an integer significand of up to 512
bits, about 154 digits, and a scale, the number of digits after the point, from
0 to 154. A number written with a decimal
point is a `Decimal`, so arithmetic on it loses nothing to binary rounding:

```hql
0.1 + 0.2 == 0.3
```

```text
true
```

The scale is kept for display. A sum or a difference has the larger scale of
its operands, and a product the sum of their scales. Equality and order are by
value, so `1.50 == 1.5` is `true`:

```hql
[1.50, 1.50 + 1.25, 1.50 * 1.5]
```

```text
[1.50, 2.75, 2.250]
```

A literal keeps every digit it is written with, however many there are:

```hql
123456789012345678901234567890.123456789 + 0.000000001
```

```text
123456789012345678901234567890.123456790
```

## Division and rounding

`/` is exact where the quotient terminates, and otherwise rounded half to even
to 100 significant digits. Division by zero is an evaluation error, as it is
for an `Int`. `round(x, places)` rounds half to even to `places` digits after
the point, and `round(x)` to a whole number:

```hql
[10 / 4.0, round(2 / 3.0, 4), round(2.345, 2), round(2.355, 2), round(2.5)]
```

```text
[2.5, 0.6667, 2.34, 2.36, 2]
```

## Int, Float and Decimal

An `Int` beside a `Decimal` widens to one of scale 0, in arithmetic and in a
comparison:

```hql
[12.50 * 3, 7 - 0.25]
```

```text
[37.50, 6.75]
```

A `Float` does not mix with a `Decimal`: a binary float carries no exact
decimal value, so the program says which conversion it means. `decimal(x)`
reads an `Int`, or a `Float` by its shortest digits, as a `Decimal`;
`float(d)` gives the nearest `Float` and `int(d)` the integer part:

```hql
1.5 + 2.0f
```

```text
check error[type-mismatch]: a Decimal and a Float do not mix: write `decimal(x)` to read the Float as a Decimal, or `float(d)` to read the Decimal as a Float
 --> <expression>:1:7
  |
1 | 1.5 + 2.0f
  |       ^^^^
```

```hql
[decimal(0.1f) + 1.5, "12.50".to_decimal().unwrap_or(0.0)]
```

```text
[1.6, 12.50]
```

JSON, YAML and CSV carry a `Decimal` without loss: a number read as a
`Decimal` field keeps the digits it is written with, and a `Decimal` is
written as a number with its digits and scale.
