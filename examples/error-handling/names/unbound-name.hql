// status: invalid
// feature: A name no binding declares is a name error
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `total` is bound and `totl` is not. The checker resolves every name over
// the whole program before any statement runs, so `totl + 1` is never
// evaluated, and `total` is never read.
total = 1
totl + 1
