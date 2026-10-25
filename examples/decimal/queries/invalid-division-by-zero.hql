// status: invalid
// feature: a Decimal divided by zero is an evaluation error, as for Int
// implementation: implemented
// environment: pure
// expected-type: Decimal
// phase: execute
// kind: division-by-zero
// severity: failure

1.50 / 0
