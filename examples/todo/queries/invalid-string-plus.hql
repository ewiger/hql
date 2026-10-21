// status: invalid
// feature: `+` on two strings, as a task line would be built
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `+` is addition on two `Int` or two `Float` operands. It does not join text,
// so a `- [ ] ` checkbox prefix cannot be added to a title this way.
"- [ ] " + "Fix broken backlinks"
