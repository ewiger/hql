// status: invalid
// feature: sort needs an order, which a declared type has only through Ord
// implementation: implemented
// environment: pure
// phase: check
// kind: unsatisfied-bound
// severity: error

// `Vec2` has no `impl Ord`, so its values have no order of their own.
// `sort(by = v => v.x)` names one; bare `sort` is rejected.
type Vec2 { x : Int, y : Int }

[Vec2(x = 2, y = 0), Vec2(x = 1, y = 0)] | sort
