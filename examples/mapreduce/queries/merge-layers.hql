// status: valid-now
// feature: `merge` combines layers of metadata, a later layer winning a key
// implementation: implemented
// environment: pure
// expected-type: Data
// expected: {genre: "novel", loan_days: 14, pages: 412, shelf: "fiction"}

// Three layers of metadata for one book: the library's defaults, what its
// genre overrides, and what the card itself says. `merge(a, b)` keeps every
// key of either and takes `b`'s value for a key in both, so folding the
// layers in order with `reduce({}, merge)` lets each layer override the ones
// before it. `merge` is associative and order-sensitive: the fold may be
// split into contiguous runs, and it refuses a Set, whose order is not a
// promise. `merge(merge(defaults, novels), dune)` is the same value.
defaults = {shelf: "general", loan_days: 21}
novels = {shelf: "fiction", loan_days: 14}
dune = {genre: "novel", pages: 412}

[defaults, novels, dune] | reduce({}, merge)
