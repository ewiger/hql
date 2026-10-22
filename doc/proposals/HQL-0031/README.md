# HQL-0031: `Decimal`, an exact decimal number type

**Status**: drafted
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

*Placeholder; not yet designed.* To be specified: the literal, its place in
the lattice (`Scalar`, `Orderable`), arithmetic and the operator traits,
precision and scale, rounding where a result has more digits than it can
keep, conversion to and from `Int`, `Float` and `String`, and how `Json`,
`Yaml` and `Csv` read and write it.

## Changelog

- 2026-10-23: drafted as a stub
