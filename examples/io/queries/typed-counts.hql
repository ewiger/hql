// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: a CSV cell is text, read as the scalar its field declares
// expected-type: Int
// expected: 4

// A CSV row is a tree of strings keyed by the header. Read as a `Tally`,
// each cell is parsed as its field's type, so `count` is an `Int` that sums.
type Tally {
    species : String
    count   : Int
}

tallies : Seq<Tally> = source(file("../counts.csv"))
tallies | map(t => t.count) | sum
