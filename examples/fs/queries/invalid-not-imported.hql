// status: invalid
// feature: `io.fs` is not in the prelude: `folder` is unbound until the program imports it
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

folder("data") | walk
