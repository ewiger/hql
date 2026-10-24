// status: valid-now
// feature: A slice of Data holding a sequence is Data
// implementation: implemented
// environment: pure
// expected-type: Option<Data>
// expected-json: [2, 1]

// A document's sequence is sliced as a List is, and the slice is Data, as
// the document it came from is. A key of a document may be missing, so
// `counts.birds` is an Option<Data>, and its slice is one too: absence
// stays absence.
counts = {birds: [1, 2, 1, 3]}

counts.birds[1:3]
