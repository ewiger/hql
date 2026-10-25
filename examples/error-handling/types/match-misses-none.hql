// status: invalid
// feature: A match over an Option covers none, or the checker rejects it
// implementation: implemented
// environment: pure
// phase: check
// kind: non-exhaustive-match
// severity: error

// `[1].first()` is `some(1)` at run time. The checker does not know that,
// and does not need to: the type is `Option<Int>`, and `none` is a member.
match [1].first() { some(n) => n }
