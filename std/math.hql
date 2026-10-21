// Module `math`: numbers, in the prelude. `abs`, `pow` and `sign` take an Int
// or a Float; `float` and `int` convert between the two, since nothing widens
// on its own; the rest take a Float. Every declaration is external: the
// `math` extension implements it.
//
// See doc/proposals/HQL-0024/parts/03-standard-library.md.

fn abs<N>(x: N) : N
fn pow<N>(x: N, exponent: N) : N
fn sign<N>(x: N) : Int
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
