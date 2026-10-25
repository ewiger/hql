// status: invalid
// feature: A match with no arm is outside the grammar
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `match` takes at least one arm. The subject `1` is never typed, since the
// parser never produces the expression.
match 1 { }
