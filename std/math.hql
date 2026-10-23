// Module `math`: numbers, in the prelude. `abs`, `pow` and `sign` take an Int
// or a Float, and `abs` and `sign` a Decimal too; `float`, `int` and
// `decimal` convert between the three, since nothing widens on its own but
// an Int beside a Decimal; `round` takes a Float or a Decimal, and the rest
// take a Float. Every declaration is external: the `math` extension
// implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md and
// doc/proposals/HQL-0031/README.md.

fn abs(self: Int) : Int
fn abs(self: Float) : Float
fn abs(self: Decimal) : Decimal
fn pow(self: Int, exponent: Int, /) : Int
fn pow(self: Float, exponent: Float, /) : Float
fn sign(self: Int) : Int
fn sign(self: Float) : Int
fn sign(self: Decimal) : Int
fn float(self: Int) : Float
// The nearest Float.
fn float(self: Decimal) : Float
fn int(self: Float) : Int
// The integer part, the fraction dropped toward zero.
fn int(self: Decimal) : Int
fn decimal(self: Int) : Decimal
// The Float by its shortest digits that read back as it.
fn decimal(self: Float) : Decimal
fn gcd(self: Int, other: Int) : Int
fn sqrt(self: Float) : Float
fn floor(self: Float) : Float
fn ceil(self: Float) : Float
fn round(self: Float) : Float
// Half to even, to `places` digits after the point, at that scale:
// round(2.345, 2) is 2.34; round(x) is round(x, 0).
fn round(self: Decimal) : Decimal
fn round(self: Decimal, places: Int) : Decimal
fn exp(self: Float) : Float
fn ln(self: Float) : Float
fn log10(self: Float) : Float
fn sin(self: Float) : Float
fn cos(self: Float) : Float
fn tan(self: Float) : Float
