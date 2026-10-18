// status: valid-now
// feature: An iterating function as a query source
// implementation: implemented
// environment: pure
// expected-type: Seq<Int>
// expected: [1, 2, 2, 4, 3, 6]

// A block body yields its values in the order each `yield` is reached. The
// declared `Seq<Int>` says production ends.
fn with_doubles(xs: List<Int>) : Seq<Int> {
    for x in xs {
        doubled = x * 2
        yield x
        yield doubled
    }
}

with_doubles([1, 2, 3])
