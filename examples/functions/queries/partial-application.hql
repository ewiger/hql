// status: valid-now
// feature: A call with a hole is a function of the arguments left open
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [19, 17]

// `_` as a whole argument leaves that argument open: `add(10, _)` is
// `n => add(10, n)`, an `Int -> Int`. Each hole takes its parameter's type.
fn add(a: Int, b: Int) : Int = a + b
fn pow(b: Int, e: Int) : Int = b * pow(b, e - 1) given e > 0 otherwise 1

square = pow(_, 2)
plus = add(_, _)

[square(3), plus(3, 4)] | map(add(10, _))
