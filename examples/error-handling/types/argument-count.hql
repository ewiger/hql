// status: invalid
// feature: A call with more arguments than the function takes is a type error
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `double` takes one argument. The call is rejected as a whole.
fn double(n: Int) : Int = n * 2
double(1, 2)
