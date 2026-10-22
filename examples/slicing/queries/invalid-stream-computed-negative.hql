// status: invalid
// feature: A Stream refuses a computed bound that counts from its end
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// error: RuntimeError
// stage: evaluate

// Checking sees only that the bound is an Int; evaluation finds it negative,
// and a position counted from the end of a Stream is not known.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

back = 0 - 3
naturals(1)[back:2]
