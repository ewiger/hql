// status: invalid
// feature: A constructor argument naming no field is a name error at the argument
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-field
// severity: error

// `Point` has `x` and no `y`. The argument is rejected where it is written.
type Point { x : Int }
Point(x = 1, y = 2)
