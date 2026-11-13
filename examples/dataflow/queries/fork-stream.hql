// status: valid-now
// feature: over a `Stream`, each branch is a stream that keeps the upstream's order
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected: [[2, 4, 6], [1, 3, 5]]

fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

{ evens, odds } = naturals(1) | fork { evens = filter(n => n % 2 == 0), odds = filter(n => n % 2 == 1) }
[evens | take(3), odds | take(3)]
