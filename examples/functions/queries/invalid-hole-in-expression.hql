// status: invalid
// feature: A hole inside an expression rather than as a whole argument
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `_` leaves a whole argument open. `_ + 1` is not an argument left open but
// an expression over nothing, so it is refused.
fn add(a: Int, b: Int) : Int = a + b

[1, 2] | map(add(_ + 1, 2))
