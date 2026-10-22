// Module `math`: numbers, in the prelude. `abs`, `pow` and `sign` take an Int
// or a Float; `float` and `int` convert between the two, since nothing widens
// on its own; the rest take a Float. Every declaration is external: the
// `math` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

fn abs(self: Int) : Int
fn abs(self: Float) : Float
fn pow(self: Int, exponent: Int, /) : Int
fn pow(self: Float, exponent: Float, /) : Float
fn sign(self: Int) : Int
fn sign(self: Float) : Int
fn float(self: Int) : Float
fn int(self: Float) : Int
fn gcd(self: Int, other: Int) : Int
fn sqrt(self: Float) : Float
fn floor(self: Float) : Float
fn ceil(self: Float) : Float
fn round(self: Float) : Float
fn exp(self: Float) : Float
fn ln(self: Float) : Float
fn log10(self: Float) : Float
fn sin(self: Float) : Float
fn cos(self: Float) : Float
fn tan(self: Float) : Float
