// status: invalid
// feature: diagnostic
// implementation: implemented
// environment: pure
// error: IntegerOverflow
// stage: evaluate
// note: Static type is Int; only evaluation overflows.
// expected-type: Int

9223372036854775807 + 1
