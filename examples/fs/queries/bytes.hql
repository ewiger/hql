// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: a file's `size` is its length in bytes, and a folder has none
// expected-type: Int
// expected: 24

import io.fs

// How many bytes the files under assets/ take.
folder("../tree/assets")
| walk
| filter(e => e.kind == "file")
| map(e => e.size | unwrap_or(0))
| sum
