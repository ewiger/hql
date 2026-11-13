// status: invalid
// feature: A bound of a range is an Int
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `a:b` builds a Range, whose bounds are positions.
[10, 20, 30]["a":2]
