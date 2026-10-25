// status: invalid
// feature: A function's body is checked against its declared result
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `name` declares `String` and its body is an `Int`. The declaration is
// rejected at the body, before any call is considered.
fn name() : String = 42
name()
