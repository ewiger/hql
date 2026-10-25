// status: invalid
// feature: A type takes as many arguments as it declares
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `List` takes one type argument. The annotation is rejected where it is
// written.
xs : List<Int, Int> = []
xs
