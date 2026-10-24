// status: valid-now
// feature: `partition(by = …)` keeps the element type and every value once; on the local backend the values keep their order
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected: [[5, 3, 8, 1], [1, 2, 3, 4]]

// One logical partition per key: odd and even numbers, then the residues
// of three. One worker holds every partition, so nothing moves.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

[
    [5, 3, 8, 1] | partition(by = x => x % 2),
    naturals(1) | partition(by = n => n % 3) | take(4)
]
