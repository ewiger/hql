// status: invalid
// feature: A receiver call no module in scope declares is a name error, with the nearest name
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `String` has `sort`, `split`, `trim` and more from the prelude's `string`
// module, and no `shout`. The call resolves by the receiver's type.
"abc".shout()
