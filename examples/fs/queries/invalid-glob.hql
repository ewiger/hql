// status: invalid
// feature: a glob pattern that does not parse is refused where `Glob` constructs it
// implementation: implemented
// environment: pure
// phase: execute
// kind: invalid-argument
// severity: failure
// expected-type: Glob

import io.fs

Glob("data/a**.csv")
