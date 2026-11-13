// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: `exists` and `entry` answer for a missing path without an error
// expected-type: List<String>
// expected: ["true", "false", "folder", "none"]

import io.fs

[
    str(Path("../tree/data").exists()),
    str(Path("../tree/missing").exists()),
    Path("../tree/data").entry() | map(e => e.kind) | unwrap_or("?"),
    str(Path("../tree/missing.csv").entry()),
]
