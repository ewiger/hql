// status: invalid
// feature: quantiles refuses fewer than two groups
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: List<Float>

from stats import quantiles

[1, 2, 3] | quantiles(1)
