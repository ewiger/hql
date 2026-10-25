// status: invalid
// feature: yield outside the block body of a user fn
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// A program has no top-level loop: `yield` is written only in the block body
// of a user `fn`.
yield 1
