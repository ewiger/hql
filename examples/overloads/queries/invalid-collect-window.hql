// status: invalid
// feature: An argument a declaration does not name is refused by the declaration
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `collect` declares one argument, the batch size `n`. A time window,
// `for = …`, is no argument of it.
[1, 2, 3] | collect(for = 5)
