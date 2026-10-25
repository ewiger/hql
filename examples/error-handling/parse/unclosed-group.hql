// status: invalid
// feature: An unclosed parenthesis is a syntax failure, before any name or type is looked at
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// The parser rejects the text at the token it cannot place. `x` is never
// resolved and `1 + 2` is never typed: there is no syntax tree to check.
x = (1 + 2
x
