// status: valid-now
// feature: `approx_distinct` estimates how many distinct values occur, deterministically
// implementation: implemented
// environment: knowledge-v1
// vault: mapreduce/wiki
// expected-type: Int
// expected: 3

// A HyperLogLog sketch: each value's SHA-256 digest picks one of 4096
// registers, and a register keeps the longest run of leading zeros it sees.
// Sketches combine by `max` per register, so a backend may count partitions
// separately, and the hash takes no seed, so every run and every backend
// gives the same estimate. Over a few values the estimate is exact; over
// thousands it is within a few percent. `Set(xs) | size` is the exact count,
// and it holds every value at once.
cards
| filter(c => c.metadata.category == "book")
| map(c => c.metadata.genre)
| approx_distinct
