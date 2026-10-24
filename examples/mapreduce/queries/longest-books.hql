// status: valid-now
// feature: `top_k(k, by)` keeps the k greatest by a key, greatest first
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["Dune", "The Common Reader", "The Hobbit"]

// `top_k` holds at most k values while it folds, and two partial lists
// combine by keeping the k greatest of both, so it is a reduction where
// `sort(by) | reverse | take(k)` is a barrier. Its longhand is the fold
// reduce(initial = [], by = (top, x) => concat(top, [x])
//        | sort(by = (a, b) => compare(by(b), by(a))) | take(k)).
// Books of equal length would keep the order they were read in.
type Book {
    title : String
    pages : Int
}

books = [
    Book(title = "Dune", pages = 412),
    Book(title = "Frankenstein", pages = 280),
    Book(title = "The Hobbit", pages = 310),
    Book(title = "A Room of One's Own", pages = 172),
    Book(title = "The Common Reader", pages = 320),
    Book(title = "Leaves of Grass", pages = 95)
]

books
| top_k(3, by = b => b.pages)
| map(b => b.title)
