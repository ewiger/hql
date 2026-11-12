// status: invalid
// feature: a stream has no last row, so no partition of one is ever complete; `over` takes a sequence
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

naturals(1) | over(order = x => x, into = (x, w) => w | row_number)
