// status: valid-now
// feature: an alias binds the function under the new name only
// implementation: implemented
// environment: pure
// expected-type: Option<Float>
// expected: 2.5

from stats import mean as arithmetic_mean

arithmetic_mean([1, 2, 3, 4])
