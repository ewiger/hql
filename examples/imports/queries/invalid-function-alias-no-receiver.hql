// status: invalid
// feature: String.shout as yell binds a function, not a receiver
// implementation: implemented
// environment: pure
// modules: imports/lib
// phase: check
// kind: unknown-name
// severity: error

from shelf import String.shout as yell

"Dune".yell()
