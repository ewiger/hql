// status: invalid
// feature: An Option is not the value it may hold
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `first()` gives `Option<Int>`, since the list may be empty. Adding to it
// is rejected: every way out of an `Option` says what absence means.
[1, 2].first() + 1
