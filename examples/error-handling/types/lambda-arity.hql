// status: invalid
// feature: A lambda takes as many parameters as the step gives it
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `map` gives one element. A lambda of two parameters is rejected.
[1] | map((a, b) => a)
