// status: valid-now
// feature: the values a predicate accepts and those it rejects are two branches of a `fork` with complementary filters
// implementation: implemented
// environment: pure
// expected-type: {large: List<Int>, small: List<Int>}
// expected: {large: [5, 8], small: [3, 1]}

[5, 3, 8, 1] | fork { large = filter(x => x > 4), small = filter(x => not (x > 4)) }
