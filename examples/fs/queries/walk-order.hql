// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: `walk` lists every entry below a folder in sorted-walk order: a folder is followed at once by what it holds
// expected-type: Seq<String>
// expected: ["2026", "2026/q1.csv", "2026-notes.md", "birds.csv"]

import io.fs

// Segments compare in turn, so `2026/q1.csv` comes before `2026-notes.md`
// although `-` sorts before `/` as text.
folder("../tree/data")
| walk
| map(e => e.path.text.trim_prefix("../tree/data/"))
