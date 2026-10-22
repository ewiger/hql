// status: proposed
// feature: A slice of Data holding a sequence is Data
// implementation: pending
// proposal: doc/proposals/HQL-0026/README.md
// environment: pure
// expected-type: Data
// expected-json: [2, 1]

// A document's sequence is sliced as a List is, and the slice is Data, as
// the document it came from is.
counts = {birds: [1, 2, 1, 3]}

counts.birds[1:3]
