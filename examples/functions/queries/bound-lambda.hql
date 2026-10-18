// status: valid-now
// feature: A lambda bound to a name, then passed to a step and piped into
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [5, 7, 9]

// A lambda written as a value types its parameters, by annotation or from the
// binding's function type, as `f` does here. Piping into a bound function
// calls it: `x | inc` is `inc(x)`.
double = (n: Int) => n * 2
f : Int -> Int = n => n + 1

[2, 3, 4] | map(double) | map(f)
