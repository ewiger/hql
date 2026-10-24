// status: invalid
// feature: A block with bindings and neither a final expression nor a yield
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// A block ends in a value or yields; one that only binds has no value.
fn one() : Int {
    x = 1
}

one()
