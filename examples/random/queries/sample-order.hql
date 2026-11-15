// status: valid-now
// feature: a sample keeps the order its elements have in the input and draws each once
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected: [true, true]

from random import sample

s = range(0, 100) | sample(10, seed = 7)
[s == sort(s), size(Set(s)) == 10]
