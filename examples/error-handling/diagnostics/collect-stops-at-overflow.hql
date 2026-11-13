// status: invalid
// feature: An overflow is fatal under collect as under strict: no element is stepped past
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// phase: execute
// kind: numeric-overflow
// severity: failure

// An overflow leaves no value to go on with, so the evaluator stops under
// both modes.
[1, 9223372036854775807] | map(n => n + 1)
