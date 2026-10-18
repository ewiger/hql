// status: valid-now
// feature: A declared function, called by name and by a pipe, and recursive
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 2, 6, 24, 120]

// `fn` declares a function with typed parameters and a result type. The name
// is bound before the body is checked, so the body may call it, and
// `given … otherwise` is what makes the recursion end.
fn factorial(n: Int) : Int = 1 given n <= 1 otherwise n * factorial(n - 1)

[1, 2, 3, 4, 5] | map(factorial)
