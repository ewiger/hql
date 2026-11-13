// status: invalid
// feature: A function named in UpperCamelCase that names no declared type is refused
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `Total` names no type this program declares, so it is no constructor, and
// a function that is not one is named in snake_case.
fn Total(xs: List<Int>) : Int = xs | sum

Total([1, 2])
