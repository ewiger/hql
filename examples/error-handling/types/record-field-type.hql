// status: invalid
// feature: A constructor argument is checked against its field's type
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `x` of `P` is an `Int`. A record is valid by construction, so the check
// is at the constructor, and no later read of `x` tests anything.
type P { x : Int }
P(x = "a")
