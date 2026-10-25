// status: invalid
// feature: A Stream refuses a literal bound counted from its end
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// A negative bound counts from the end, and a Stream may not end, so
// `[-3:]` names positions that are not known.
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}

naturals(1)[-3:]
