// status: invalid
// feature: yield inside a lambda
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// A lambda body is one expression, never a block, so it cannot yield.
[1, 2] | map(x => yield x)
