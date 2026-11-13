// status: invalid
// feature: An index is an Int position or a Range
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `xs[i]` resolves through `Index`, and a `List` is indexed by position.
// A `String` index is rejected at the bracket.
[1, 2, 3]["a"]
