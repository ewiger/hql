// status: invalid
// feature: `into` gives no window: a window is a view of its partition during the call
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

[1, 2, 3] | over(order = x => x, into = (x, w) => [w])
