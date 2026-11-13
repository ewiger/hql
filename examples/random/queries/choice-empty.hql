// status: valid-now
// feature: choice over an empty sequence is none
// implementation: implemented
// environment: pure
// expected-type: Option<Int>
// expected: none

import random

[1, 2, 3] | filter(n => n > 5) | choice(seed = 1)
