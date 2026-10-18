// status: valid-now
// feature: take ends a stream that would not end by itself
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 2, 3, 4, 5]

// `naturals` never stops yielding. `take(5)` asks for five values and then
// stops asking, so the traversal ends there.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

naturals(1) | take(5)
