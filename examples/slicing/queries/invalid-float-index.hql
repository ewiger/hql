// status: invalid
// feature: An index is an Int position or a Range
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// The type inside the bracket decides between a position and a slice, and
// a Float is neither.
[10, 20, 30][1.5f]
