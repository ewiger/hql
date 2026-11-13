// status: invalid
// feature: a Decimal and a Float do not mix: the conversion is written
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// A binary float carries no exact decimal value, so `1.5 + 2.0f` is refused
// naming `decimal(x)` and `float(d)`.
1.5 + 2.0f
