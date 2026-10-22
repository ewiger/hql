// Module `math`: numbers, in the prelude. `abs`, `pow` and `sign` take an Int
// or a Float; `float` and `int` convert between the two, since nothing widens
// on its own; the rest take a Float. Every declaration is external: the
// `math` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

fn abs(x: Int) : Int
fn abs(x: Float) : Float
fn pow(x: Int, exponent: Int) : Int
fn pow(x: Float, exponent: Float) : Float
fn sign(x: Int) : Int
fn sign(x: Float) : Int
fn float(x: Int) : Float
fn int(x: Float) : Int
fn gcd(x: Int, other: Int) : Int
fn sqrt(x: Float) : Float
fn floor(x: Float) : Float
fn ceil(x: Float) : Float
fn round(x: Float) : Float
fn exp(x: Float) : Float
fn ln(x: Float) : Float
fn log10(x: Float) : Float
fn sin(x: Float) : Float
fn cos(x: Float) : Float
fn tan(x: Float) : Float
