// status: valid-now
// feature: `+` on two strings, as a task line would be built
// implementation: implemented
// environment: pure
// expected-type: String
// expected: - [ ] Fix broken backlinks

// `+` on two `String` operands is concatenation: `String` implements `Add`.
// Both operands are strings; `"- [ ] " + 1` is a type error.
"- [ ] " + "Fix broken backlinks"
