// status: invalid
// feature: An argument a declaration does not name is refused by the declaration
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `collect` declares one argument, the batch size `n`. A time window,
// `for = …`, is no argument of it.
[1, 2, 3] | collect(for = 5)
