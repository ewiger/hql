// status: valid-now
// feature: the version 7 UUIDs of one evaluation sort in the order they were drawn
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected: [true, true]

import random

ids = range(0, 20) | map(_ => uuid())
[ids == sort(ids), size(Set(ids)) == 20]
