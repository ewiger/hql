// status: valid-now
// feature: map, filter and drop over a stream, one value at a time
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [36, 64, 100]

fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

// Each step keeps the stream a stream; only `take` bounds it.
naturals(1)
| filter(n => n % 2 == 0)
| map(n => n * n)
| drop(2)
| take(3)
