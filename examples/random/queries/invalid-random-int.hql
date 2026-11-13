// status: invalid
// feature: random_int refuses a high bound below the low one
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Int

import random

random_int(6, 1)
