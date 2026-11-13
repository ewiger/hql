// status: valid-now
// feature: a set is sorted before it is sampled, so two orders of building it give one sample
// implementation: implemented
// environment: pure
// expected-type: Bool
// expected: true

import random

samples = [Set([5, 3, 9, 1, 7]), Set([7, 1, 9, 3, 5])] | map(s => s | sample(2, seed = 11))
samples[0] == samples[1]
