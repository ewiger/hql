// status: proposed
// feature: a Decimal keeps every digit it is written with
// implementation: pending
// proposal: doc/proposals/HQL-0031/README.md
// environment: pure
// expected-type: Decimal
// expected: 123456789012345678901234567890.123456790

// Thirty-nine significant digits, more than a Float holds, and the sum is
// exact in every one of them.
123456789012345678901234567890.123456789 + 0.000000001
