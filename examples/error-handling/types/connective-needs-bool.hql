// status: invalid
// feature: A logical connective takes Bool operands
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `and` is not defined over `Int`: no value is truthy, and `1 and true` has
// no meaning to compute.
1 and true
