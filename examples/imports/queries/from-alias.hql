// status: proposed
// feature: an alias binds the function under the new name only
// implementation: pending
// proposal: doc/proposals/HQL-0060/index.hmd
// environment: pure
// expected-type: Option<Float>
// expected: 2.5

from stats import mean as average

average([1, 2, 3, 4])
