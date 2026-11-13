// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: an entry's `path` is one `file` reads as written
// expected-type: List<Option<Data>>
// expected: ["heron", "owl"]

import io.fs

// Every row of every CSV file directly in data/, read through `file`.
folder("../tree/data")
| glob(Glob("*.csv"))
| flat_map(e => source(file(e.path), Csv))
| map(row => row.species)
