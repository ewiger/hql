// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: `glob` with `**` matches at any depth, and an entry's `size` sorts the files
// expected-type: Seq<String>
// expected: ["birds.csv", "q1.csv"]

import io.fs

// The CSV files under data/, largest first.
folder("../tree/data")
| glob(Glob("**/*.csv"))
| sort(by = e => e.size | unwrap_or(0))
| reverse
| map(e => e.name)
