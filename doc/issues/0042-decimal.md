# 0042 — Decimal

Status: done
Related: [HQL-0031](../proposals/HQL-0031/README.md)

A fundamental `Decimal` type, beside `Int`, `Float` and `String`: a number
with a decimal point that is exact, not floating point, so a long number
keeps every digit and arithmetic loses no precision. For financial
applications and anywhere an amount must stay what it was written as.

## Done when

- HQL-0031 is specified and accepted.
- `Decimal` is in the lattice with its literal, arithmetic, comparison and
  conversions, and the formats read and write it without loss.

## Done

- HQL-0031 is accepted and implemented. A number with a decimal point is a
  `Decimal` and `f` writes a `Float`; `Decimal` is `Scalar` and `Orderable`
  with `+ - * / %`, the comparisons, `min`, `max`, `sum`, `product`,
  `average`, `abs`, `sign` and half-even `round`; an `Int` widens to it and a
  `Float` beside it is a type error naming `decimal(x)` and `float(d)`;
  `decimal`, `float`, `int` and `to_decimal` convert. JSON, YAML and CSV read
  a `Decimal` field with its digits and write it with its scale, shown in
  [the decimal examples](../../examples/decimal/README.md).
- A format boundary declaring a decimal field as a JSON string is
  [HQL-0035](../proposals/HQL-0035/README.md)'s schema.
