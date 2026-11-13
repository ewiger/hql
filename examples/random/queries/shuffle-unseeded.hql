// status: valid-now
// feature: an unseeded shuffle draws from the evaluation seed and keeps every element
// implementation: implemented
// environment: pure
// expected-type: Bool
// expected: true

import random

(range(0, 29) | shuffle | sort) == range(0, 29)
