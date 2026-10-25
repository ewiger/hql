// status: invalid
// feature: A function's parameters are distinct, and the parser says so
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// Two parameters named `x` are rejected where the second is written. No
// scope is built, so this is not a name error.
fn f(x: Int, x: Int) : Int = x
f(1, 2)
