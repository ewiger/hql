# HQL-0031: `Decimal`, an exact decimal number type

**Status**: accepted
**Created**: 2026-10-23
**Source**: [issue 0042](../../issues/0042-decimal.md);
[HQL-0030](../HQL-0030/README.md) uses `Decimal` in its examples

## Abstract

`Decimal` is a fundamental type beside `Int`, `Float` and `String`: a number
written with a decimal point whose value is exact. It is not a floating-point
number, so arithmetic on it never loses precision to binary rounding, and
long numbers keep every digit they are written with. It is the type for
financial and other applications where an amount must be what it was written
as.

## Motivation

`Float` is a binary floating-point number: `0.1 + 0.2` is not `0.3`, and an
amount with many digits is rounded. Money, quantities and identifiers written
as long numbers need a type whose value is exactly its digits.

## Specification

### 1. The literal

A number written with a decimal point is a `Decimal`. A `Float` is written
with the suffix `f`.

```hql
price = 12.50        // Decimal, scale 2
rate = 0.1 + 0.2     // Decimal 0.3, exactly
big = 123456789012345678901234567890.123456789   // every digit kept
ratio = 0.5f         // Float
huge = 6.02e23f      // Float
milli = 1.5e-3       // Decimal 0.0015
n = 12               // Int, unchanged
```

- A literal of digits, a `.`, digits, and an optional exponent `e` or `E`
  with an optional sign is a `Decimal`. Its **scale** is the number of
  digits after the point, less the exponent where one is written, and never
  below 0: `12.50` has scale 2, `1.5e-3` scale 4, `1.5e3` scale 0.
- The same literal followed directly by `f` is a `Float`. An integer
  literal followed by `f`, `2f`, is a `Float` too.
- A literal of digits only stays an `Int`.

### 2. Precision

A `Decimal` is an arbitrary-precision decimal: an integer significand of
any size and a scale.

- The significand MUST be unbounded by the representation; an
  implementation MUST NOT limit it below 256 bits (77 decimal digits).
- Every literal, sum, difference and product is exact.

### 3. Type and lattice

`Decimal` is a scalar beside `Int`, `Float`, `String` and `Bool`.

- It is `Scalar`, `Orderable` and numeric: it has `+`, `-`, `*`, `/`,
  unary `-`, the comparisons, `min`, `max`, `sum`, `avg` and `abs`, as
  `Int` and `Float` have them.
- Equality and order are by value: `1.0 == 1.00` is `true`. The scale is
  kept for display: `1.50` prints as `1.50`.

### 4. Arithmetic and rounding

Each operation states its scale, so results are reproducible.

- `a + b` and `a - b` have the larger scale of the two. `a * b` has the sum
  of their scales.
- `a / b` is exact where the quotient terminates. Where it does not, it is
  rounded half-even to 100 significant digits. Division by zero is an
  evaluation error, as for `Int`.
- `round(x, places)` rounds `x` half-even to `places` digits after the
  point and has scale `places`: `round(2.345, 2)` is `2.34`,
  `round(2.355, 2)` is `2.36`. `round(x)` is `round(x, 0)`.

### 5. Mixing with `Int` and `Float`

`Int` widens to `Decimal`. `Float` does not mix.

- In an operation or comparison with one `Decimal` and one `Int`, the `Int`
  is read as a `Decimal` of scale 0: `1.5 + 2` is `3.5`.
- An operation or comparison with one `Decimal` and one `Float` is refused
  when the program is checked, because a binary float carries no exact
  decimal value. The conversion is written: `decimal(x)` reads an `Int` or
  a `Float` as a `Decimal`, the `Float` by its shortest exact digits, and
  `float(d)` gives the nearest `Float`, `int(d)` the integer part.
- `s.to_decimal()` reads a `String` as an `Option<Decimal>`, as
  `to_float` and `to_int` do.
- `string(d)` and `fmt` write the digits with the value's scale.

### 6. Formats

A format carries a `Decimal` without loss.

- Reading a JSON or YAML number as `Decimal` keeps its digits exactly as
  written, and reading a CSV cell as `Decimal` reads its text as a
  `Decimal` literal.
- Writing a `Decimal` to JSON or YAML writes a number with its digits and
  scale; to CSV and text, its digits.
- A format boundary MAY declare otherwise in its schema: a decimal field
  carried as a JSON string, for a consumer that reads JSON numbers as
  binary floats. [HQL-0035](../HQL-0035/README.md) specifies the
  boundary's schema.

## Backwards Compatibility

Every unsuffixed literal with a decimal point changes type from `Float` to
`Decimal`. A program that needs a `Float`, to pass to a function declared
over `Float` or to mix with one, writes the suffix `f`. Programs that only
compute with such literals give the same digits, now exact.

## Changelog

- 2026-10-23: drafted as a stub
- 2026-10-23: accepted: a bare decimal literal is `Decimal` and `Float` takes
  `f`; arbitrary precision; exact `+ - *`, division to 100 significant
  digits, half-even `round`; `Int` widens and `Float` is refused; formats
  carry the digits exactly
