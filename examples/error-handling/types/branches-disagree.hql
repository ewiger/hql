// status: invalid
// feature: Both branches of a conditional meet the declared result
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `1` and `"one"` meet only at `Data`, and `pick` declares `Int`. The call
// `pick(true)` would take the `Int` branch; the checker rejects the
// function regardless, since the result is decided by both branches.
fn pick(flag: Bool) : Int = 1 given flag otherwise "one"
pick(true)
