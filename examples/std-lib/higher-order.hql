// status: proposed
// feature: generic higher-order function
// implementation: pending
// environment: pure
// expected-type: Int
// expected: 42
// note: Functions are ordinary values and A -> B is an ordinary type, so a generic function needs no trait and no bound. Traits are for requiring a surface of a type; parametric polymorphism covers everything that needs no surface at all.

fn apply<A, B>(f : A -> B, x : A) -> B = f(x)

apply(n => n + 1, 41)
