// status: invalid
// feature: A block that yields and also ends in an expression
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// A block containing `yield` is iterating, and an expression on a line of its
// own does not appear in one.
fn numbers() : Seq<Int> {
    yield 1
    2
}

List(numbers())
