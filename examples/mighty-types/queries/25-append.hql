// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: Append to a file's end, one record per value, as it arrives
// expected-type: Unit
// expected: ()

// `file(p).end` is where an append goes: `sink` writes one record per value
// after what the file holds, and never replaces it. Each run adds three more
// lines to `data/journal.txt`; a program that ends in a write has no value
// to show.
source(file("../data/haiku.txt"), Lines)
| map(l => fmt("{} ({} characters)", l, l.length()))
| sink(file("../data/journal.txt").end, Lines)
