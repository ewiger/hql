// status: valid-now
// feature: round(x, places) rounds a Decimal half to even, at scale places
// implementation: implemented
// environment: pure
// expected-type: List<Decimal>
// expected: [2.34, 2.36, 2, 4, 1.200]

// A half goes to the even neighbour: 2.345 to 2.34 and 2.355 to 2.36.
// `round(x)` is `round(x, 0)`; more places than the value has add zeros.
[round(2.345, 2), round(2.355, 2), round(2.5), 3.5.round(), round(1.2, 3)]
