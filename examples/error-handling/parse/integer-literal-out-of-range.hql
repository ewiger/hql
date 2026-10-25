// status: invalid
// feature: A literal past the Int range is a syntax failure, where an overflow at evaluation is not
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `9223372036854775808` is one past what an `Int` holds, and the text itself
// is rejected. Compare `evaluation/overflow.hql`: `9223372036854775807 + 1`
// is well typed, and the overflow is an evaluation failure of the run.
n = 9223372036854775808
n
