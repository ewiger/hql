// status: invalid
// feature: An Option is not the value it may hold
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `first()` gives `Option<Int>`, since the list may be empty. Adding to it
// is rejected: every way out of an `Option` says what absence means.
[1, 2].first() + 1
