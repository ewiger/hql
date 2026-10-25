// status: invalid
// feature: A receiver call no module in scope declares is a name error, with the nearest name
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `String` has `sort`, `split`, `trim` and more from the prelude's `string`
// module, and no `shout`. The call resolves by the receiver's type.
"abc".shout()
