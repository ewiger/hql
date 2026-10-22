// status: valid-now
// feature: A bounded slice of a Stream is a List, and an open one is a Stream
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected-json: [[1, 2, 3], [11, 12, 13], [101, 102]]

// A Stream may not end, so what a slice of one is depends on its bounds.
// `xs[:n]` is `take(n)` and `xs[a:b]` is `drop(a) | take(b - a)`: both end,
// so both are Lists. `xs[a:]` is `drop(a)` and stays a Stream, which a
// further bounded slice ends.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

[
    naturals(1)[:3],
    naturals(1)[10:13],
    naturals(1)[100:][:2],
]
