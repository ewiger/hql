// status: invalid
// feature: quantile refuses a q outside 0 to 1
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Option<Float>

from stats import quantile

[1, 2, 3] | quantile(1.5)
