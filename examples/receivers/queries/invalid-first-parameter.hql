// status: invalid
// feature: A first parameter is never a receiver
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `fn range(start: Int, end: Int, step: Int = 1)` declares no `self`, so
// `range(1, 3)` and `1 | range(3)` reach it and `1.range(3)` does not.
1.range(3)
