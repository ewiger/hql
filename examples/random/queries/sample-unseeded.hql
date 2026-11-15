// status: valid-now
// feature: an unseeded sample draws from the evaluation seed and has n elements
// implementation: implemented
// environment: pure
// expected-type: Int
// expected: 3

from random import *

range(0, 1000) | sample(3) | size
