// status: invalid
// feature: An argument is checked against the parameter it fills
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `double` takes an `Int`, and `"2"` is a `String`. The diagnostic names the
// parameter.
fn double(n: Int) : Int = n * 2
double("2")
