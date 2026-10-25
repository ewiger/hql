// status: invalid
// feature: `for` in a value block
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// `for` belongs to an iterating block. A value block is bindings and a final
// expression.
fn total(xs: List<Int>) : Int {
    for x in xs {
        y = x
    }
    0
}

total([1, 2])
