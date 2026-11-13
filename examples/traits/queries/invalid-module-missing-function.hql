// status: invalid
// feature: A module listing a trait declares each function the trait requires without a default body
// implementation: implemented
// environment: pure
// modules: traits/lib
// phase: check
// kind: missing-implementation
// severity: error

// `shapes.square` lists `Shape` and declares no `name`. The refusal is
// against its module line, and names the trait and the function.
import shapes.square

Square(side = 2.0f).area()
