// status: proposed
// feature: a comparison of an Int with a Decimal widens the Int, and compares by value
// implementation: pending
// proposal: doc/proposals/HQL-0031/README.md
// environment: pure
// expected-type: List<Bool>
// expected: [true, true, true, true]

// Equality and order are by value: `1.0 == 1.00`, whatever the scales.
[12.5 < 13, 1.0 == 1, 2 >= 2.00, 1.0 == 1.00]
