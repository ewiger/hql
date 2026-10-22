# 0042 — Decimal

Status: backlog
Related: [HQL-0031](../proposals/HQL-0031/README.md)

A fundamental `Decimal` type, beside `Int`, `Float` and `String`: a number
with a decimal point that is exact, not floating point, so a long number
keeps every digit and arithmetic loses no precision. For financial
applications and anywhere an amount must stay what it was written as.

## Done when

- HQL-0031 is specified and accepted.
- `Decimal` is in the lattice with its literal, arithmetic, comparison and
  conversions, and the formats read and write it without loss.
