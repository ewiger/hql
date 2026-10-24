// status: invalid
// feature: `= { … }` written as a body over a non-record result
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `= {` begins a record or set literal, never a block, so the binding line
// inside it does not parse.
fn one() : Int = {
    x = 1
    x
}

one()
