// status: valid-now
// feature: `histogram(width)` counts values per bucket, in key order
// implementation: implemented
// environment: pure
// expected-type: SortedMap<Int, Int>
// expected: SortedMap {0: 1, 100: 1, 200: 1, 300: 2, 400: 1}

// Each page count falls in the bucket whose lower bound is the greatest
// multiple of the width not above it: 412 in 400, 95 in 0. The buckets are a
// SortedMap, so they come back in key order whatever order the books were
// read in, and two partial histograms combine by `+` per bucket. A width of
// zero or less is an evaluation error.
pages = [412, 280, 310, 172, 320, 95]

pages | histogram(100)
