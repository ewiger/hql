// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Closures capture, compose, repeat and apply partially
// expected-type: List<Int>
// expected: [8, 8, 15, 15, 15, 14]

// A lambda keeps the bindings in scope where it was written. `compose` is
// generic over three types, `_` leaves an argument open, and `+` named as
// a whole argument is the function that adds.
fn compose<A, B, C>(f: A -> B, g: B -> C) : A -> C = x => g(f(x))
fn apply_n(f: Int -> Int, n: Int, x: Int) : Int = x given n == 0 otherwise apply_n(f, n - 1, f(x))
fn counter(start: Int) : Int -> Int = step => start + step

inc = (n: Int) => n + 1
double = (n: Int) => n * 2
inc_then_double = compose(inc, double)
from_ten = counter(10)
add = (a: Int, b: Int) => a + b
add_five = add(5, _)

[
    inc_then_double(3),
    apply_n(double, 3, 1),
    from_ten(5),
    add_five(10),
    [1, 2, 3] | map(compose(double, inc)) | sum,
    [1, 2, 3].map(x => x * x).reduce(0, +),
]
