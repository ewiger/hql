// status: invalid
// feature: A well-typed addition that leaves the Int range is an evaluation failure
// implementation: implemented
// environment: pure
// expected-type: Int
// phase: execute
// kind: numeric-overflow
// severity: failure

// The program checks: both operands are `Int`, and `hql check` prints `Int`.
// The sum depends on the values, and the run reports it. Compare
// `parse/integer-literal-out-of-range.hql`, where the text itself is
// rejected.
x = 9223372036854775807 + 1
x
