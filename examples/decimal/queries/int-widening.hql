// status: proposed
// feature: an Int beside a Decimal widens to a Decimal of scale 0
// implementation: pending
// proposal: doc/proposals/HQL-0031/README.md
// environment: pure
// expected-type: List<Decimal>
// expected: [37.50, 2.5, 6.75]

// `price * qty` is a Decimal, and `10 / 4.0` divides exactly.
price = 12.50
qty = 3
[price * qty, 10 / 4.0, 7 - 0.25]
