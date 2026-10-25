// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md and
// doc/proposals/HQL-0031/README.md.

/// Numbers, in the prelude. `abs`, `pow` and `sign` take an `Int` or a
/// `Float`, and `abs` and `sign` a `Decimal` too; `round` takes a `Float` or
/// a `Decimal`, and the rest take a `Float`. `float`, `int` and `decimal`
/// convert between the three, since no number widens on its own except an
/// `Int` beside a `Decimal`.
module math

import core

/// `self` without its sign, at its own type: `abs(-3)` is `3`, and
/// `abs(-1.50)` is `1.50`.
fn abs(self: Int) : Int
fn abs(self: Float) : Float
fn abs(self: Decimal) : Decimal

/// `self` raised to `exponent`, both of one type: `pow(2, 10)` is `1024`.
/// Over `Int`s an `exponent` below 0 is an evaluation error, and so is a
/// result past the range of `Int`.
fn pow(self: Int, exponent: Int, /) : Int
fn pow(self: Float, exponent: Float, /) : Float

/// `-1`, `0` or `1` by the sign of `self`, as an `Int`: `sign(-0.5)` is
/// `-1`.
fn sign(self: Int) : Int
fn sign(self: Float) : Int
fn sign(self: Decimal) : Int

/// `self` as a `Float`, the `Float` nearest it: `float(1.10)` is `1.1`.
fn float(self: Int) : Float
fn float(self: Decimal) : Float

/// The integer part of `self`, as an `Int`, the fraction dropped toward
/// zero: `int(-2.9f)` is `-2`. A value past the range of `Int` is an
/// evaluation error.
fn int(self: Float) : Int
fn int(self: Decimal) : Int

/// `self` as a `Decimal`: an `Int` exactly, and a `Float` by the shortest
/// digits that read back as it: `decimal(0.1f)` is `0.1`.
fn decimal(self: Int) : Decimal
fn decimal(self: Float) : Decimal

/// The greatest common divisor of `self` and `other`, at least 0:
/// `gcd(12, 18)` is `6`, and `gcd(0, 0)` is `0`.
fn gcd(self: Int, other: Int) : Int

/// The square root of `self`: `sqrt(2.0f)` is `1.4142135623730951`. A result
/// that is not a finite number, as for a negative `self`, is an evaluation
/// error.
fn sqrt(self: Float) : Float

/// The greatest whole number not above `self`, as a `Float`:
/// `floor(-1.5f)` is `-2.0`.
fn floor(self: Float) : Float

/// The least whole number not below `self`, as a `Float`: `ceil(-1.5f)` is
/// `-1.0`.
fn ceil(self: Float) : Float

/// The whole number nearest `self`, as a `Float`; a value halfway between
/// two rounds away from zero: `2.5f | round` is `3.0`.
fn round(self: Float) : Float

/// `self` rounded half to even to `places` digits after the point, and
/// kept at that scale: `round(2.345, 2)` is `2.34`, and `round(x)` is
/// `round(x, 0)`. See [[/learn/02-data-types/decimal#division-and-rounding|Division and rounding]].
fn round(self: Decimal) : Decimal
fn round(self: Decimal, places: Int) : Decimal

/// e raised to `self`: `exp(1.0f)` is `2.718281828459045`. A result that is
/// not a finite number is an evaluation error.
fn exp(self: Float) : Float

/// The natural logarithm of `self`: `ln(1.0f)` is `0.0`. A result that is
/// not a finite number, as for a `self` of 0 or below, is an evaluation
/// error.
fn ln(self: Float) : Float

/// The base-10 logarithm of `self`: `log10(1000.0f)` is `3.0`. A result
/// that is not a finite number is an evaluation error.
fn log10(self: Float) : Float

/// The sine of `self`, an angle in radians: `sin(0.0f)` is `0.0`.
fn sin(self: Float) : Float

/// The cosine of `self`, an angle in radians: `cos(0.0f)` is `1.0`.
fn cos(self: Float) : Float

/// The tangent of `self`, an angle in radians: `tan(0.0f)` is `0.0`.
fn tan(self: Float) : Float
