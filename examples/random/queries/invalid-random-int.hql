// status: invalid
// feature: random_int refuses a high bound below the low one
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Int

from random import random_int

random_int(6, 1)
