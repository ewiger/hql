// status: valid-now
// feature: A value block: bindings followed by the expression that is the function's value
// implementation: implemented
// environment: pure
// expected-type: List<Float>
// expected-json: [6.0, 12.5]

// A body written `{ … }` without `=` is a block. Each `name = expr` line
// binds a name for the lines after it, and the last line is an expression:
// its value is the function's value and has the declared result type.
fn area(width: Float, height: Float) : Float {
    w = width
    h = height
    w * h
}

fn triangle(base: Float, height: Float) : Float {
    whole = area(base, height)
    whole / 2.0f
}

[area(2.0f, 3.0f), triangle(5.0f, 5.0f)]
