// status: proposed
// feature: a Decimal keeps its scale for display and compares by value
// implementation: pending
// proposal: doc/proposals/HQL-0031/README.md
// environment: pure
// expected-type: List<Decimal>
// expected: [1.50, 2.75, 2.250, 0.02]

// A sum has the larger scale of its operands and a product the sum of
// theirs; `1.50 == 1.5` holds, and each prints with its own digits.
[1.50, 1.50 + 1.25, 1.50 * 1.5, 0.1 * 0.2]
