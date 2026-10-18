// status: invalid
// feature: yield outside the block body of a user fn
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// A program has no top-level loop: `yield` is written only in the block body
// of a user `fn`.
yield 1
