// status: valid-now
// feature: a number written with a decimal point is an exact Decimal
// implementation: implemented
// environment: pure
// expected-type: Bool
// expected: true

// `0.1` and `0.2` are Decimals, so their sum is exactly `0.3`. A Float,
// `0.1f + 0.2f`, is a binary fraction and is not.
0.1 + 0.2 == 0.3
