// status: invalid
// feature: a Decimal divided by zero is an evaluation error, as for Int
// implementation: implemented
// environment: pure
// expected-type: Decimal
// error: RuntimeError
// stage: evaluate

1.50 / 0
