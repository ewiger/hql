// status: invalid
// feature: a join states when elements match; every pair is asked for as cross_join
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

[1, 2] | join([2, 3], into = (x, y) => x + y)
