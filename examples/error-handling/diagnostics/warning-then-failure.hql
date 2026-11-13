// status: invalid
// feature: A check warning and an execute failure travel on one channel, in the order they were written
// implementation: implemented
// environment: pure
// expected-type: Int
// phase: execute
// kind: division-by-zero
// severity: failure

// Checking writes a `naming-convention` warning for `totalOf`, and the
// program still runs: a warning never stops a phase. Executing then divides
// by zero. Both diagnostics are on the run's diagnostic channel, the warning
// first.
fn totalOf(xs : List<Int>) : Int = xs | sum

totalOf([1, 2, 3]) / 0
