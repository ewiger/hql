// status: valid-now
// feature: A value block whose result is a Seq, without yield
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [4, 16]

// A value block takes any result type, `Seq<T>` included. Without `yield`
// the block is not iterating: its last line is the sequence it returns.
fn even_squares(xs: List<Int>) : Seq<Int> {
    evens = xs | filter(x => x % 2 == 0)
    evens | map(x => x * x)
}

List(even_squares([1, 2, 3, 4]))
