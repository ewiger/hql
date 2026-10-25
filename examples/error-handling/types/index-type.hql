// status: invalid
// feature: An index is an Int position or a Range
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `xs[i]` resolves through `Index`, and a `List` is indexed by position.
// A `String` index is rejected at the bracket.
[1, 2, 3]["a"]
