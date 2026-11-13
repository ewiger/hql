// status: valid-now
// feature: a seeded shuffle is a permutation of its input
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 2, 3, 4, 5]

import random

[3, 1, 4, 5, 2] | shuffle(seed = 42) | sort
