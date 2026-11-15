// status: invalid
// feature: a negative sample size is an evaluation error
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: List<Int>

from random import sample

[1, 2, 3] | sample(-1, seed = 1)
