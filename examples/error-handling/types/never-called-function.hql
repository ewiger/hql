// status: invalid
// feature: A function nobody calls is checked all the same
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `never` is declared and not called; the program's value would be `42`.
// Its body does not check, so the program has no value: checking covers
// the whole program, not the path a run takes through it.
fn never() : Int = "a" + 1
42
