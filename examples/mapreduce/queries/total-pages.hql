// status: proposed
// feature: A fold with `reduce` over a sequence
// implementation: pending
// proposal: doc/proposals/HQL-0004/README.md
// environment: pure
// expected-type: Int
// expected: 1589

// `reduce` is a left fold: by(by(by(0, 412), 280), 310) and so on. It takes a
// Seq because a fold visits positions in order; a Set has none, and would have
// to be sorted first. The combining function is the program's own, so no
// backend may split it across partitions — it is not known to be associative.
// `size` and `count` are the reductions whose combiner the executor does know.
pages = [412, 280, 310, 172, 320, 95]

pages
| reduce(0, (total, n) => total + n)
