// status: invalid
// feature: An operator over two types with no common form is a type error
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `+` resolves through `Add`, and `Add` for `String` takes a `String`. The
// program has no value to report, since it never runs.
"a" + 1
