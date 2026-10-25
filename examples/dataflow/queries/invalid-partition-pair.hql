// status: invalid
// feature: `partition` is no longer a predicate's split into a pair: it keeps its input's type
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-field
// severity: error

([5, 3, 8, 1] | partition(by = x => x > 4)).first
