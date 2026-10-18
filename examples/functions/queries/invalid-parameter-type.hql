// status: invalid
// feature: A function given an argument its parameter does not take
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `double` takes an Int. A String is not one, and the checker names the
// argument and the parameter type rather than failing at run time.
fn double(n: Int) : Int = n * 2

double("two")
