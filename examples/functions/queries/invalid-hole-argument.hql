// status: invalid
// feature: An argument that does not fit the parameter beside a hole
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// The hole leaves `b` open; `a` is fixed to a String, which `add` does not
// take, and the diagnostic names the parameter.
fn add(a: Int, b: Int) : Int = a + b

[1, 2] | map(add("ten", _))
