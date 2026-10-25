// status: invalid
// feature: A divisor that is zero for one element is an evaluation failure of the run
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// error: RuntimeError
// stage: evaluate

// `10 / (n - 2)` is `Int / Int`, which checks. For `n = 2` the divisor is
// zero, a fact about the value, and the run fails there.
[1, 2, 3] | map(n => 10 / (n - 2))
