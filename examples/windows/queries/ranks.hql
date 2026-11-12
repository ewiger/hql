// status: proposed
// feature: `row_number`, `rank` and `dense_rank` over a partition with a tie
// implementation: pending
// proposal: doc/proposals/HQL-0050/README.md
// environment: pure
// expected-type: List<List<Int>>
// expected: [[1, 1, 1], [3, 3, 2], [2, 1, 1], [4, 4, 3]]

// Ordered by score, highest first: a and c tie at 9, a first by input
// position. Each row's ranks are in the row's own place.
type Score {
    name  : String
    score : Int
}

scores = [
    Score(name = "a", score = 9),
    Score(name = "b", score = 7),
    Score(name = "c", score = 9),
    Score(name = "d", score = 5),
]
scores
| over(order = (x, y) => compare(y.score, x.score),
       into = (s, w) => [w | row_number, w | rank, w | dense_rank])
