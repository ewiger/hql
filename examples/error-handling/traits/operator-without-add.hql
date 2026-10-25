// status: invalid
// feature: An operator resolves through its core trait, and a type without the impl has no operator
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `+` is `Add`, and `Vec2` implements it nowhere. The checker rejects the
// addition; `impl Add for Vec2` would give the type its operator.
type Vec2 { x : Int, y : Int }

Vec2(x = 1, y = 2) + Vec2(x = 3, y = 4)
