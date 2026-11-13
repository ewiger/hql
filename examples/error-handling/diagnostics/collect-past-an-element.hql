// status: invalid
// feature: Under collect, map steps past an element whose function failed, and the diagnostic says so with disposition recoverable
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// phase: execute
// kind: division-by-zero
// severity: failure

// Under `strict` the division by zero ends the run, `disposition: fatal`.
// Under `collect` the evaluator drops the one element, writes the same
// diagnostic with `disposition: recoverable`, and gives `[1, 2]`; the exit
// status is still `1`, since a `failure` was written.
[6, 0, 3] | map(n => 6 / n)
