// status: valid-now
// feature: `merge`, `zip` and `concat` over streams: one from each input in turn, pairs by position, and one input after the other
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["1", "10", "2", "11", "3", "12"], ["1a", "2b", "3a"], ["100", "200", "1", "2"]]

// The local backend's merge takes one value from each input in turn,
// skipping one that has ended; each input keeps its own order.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

fn letters() : Stream<String> {
    yield "a"
    yield "b"
    for l in letters() {
        yield l
    }
}

fn hundreds() : Seq<Int> {
    yield 100
    yield 200
}

[
    naturals(1) | merge(naturals(10)) | map(n => str(n)) | take(6),
    naturals(1) | zip(letters()) | map(p => fmt("{}{}", p.first, p.second)) | take(3),
    hundreds() | concat(naturals(1)) | map(n => str(n)) | take(4)
]
