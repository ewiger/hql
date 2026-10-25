// status: invalid
// feature: A predicate's type is checked with no element to apply it to
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `xs` is empty, so `filter` would never call the lambda. The lambda's body
// is an `Int` where `filter` needs a `Bool`, and the checker rejects it:
// the type of the step is decided from the types, not from the elements.
xs : List<Int> = []
xs | filter(n => n + 1)
