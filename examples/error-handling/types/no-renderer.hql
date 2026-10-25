// status: invalid
// feature: A format that cannot write a type is a type error, not a run-time surprise
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `D2` draws graphs. Nothing writes an `Int` as a diagram, and the checker
// says so at the step.
1 | d2
