// status: invalid
// feature: yield inside a lambda
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// A lambda body is one expression, never a block, so it cannot yield.
[1, 2] | map(x => yield x)
