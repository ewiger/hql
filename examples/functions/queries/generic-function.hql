// status: valid-now
// feature: A generic function takes its types from each call
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 3]

// `T` stands for whatever the argument's element is: `first([3, 1])` is an
// `Option<Int>`. A bound, `T : Orderable`, is checked at each call, and the
// body may rely on it.
fn first<T>(xs: List<T>) : Option<T> = xs | reduce((a, b) => a)
fn smallest<T : Orderable>(xs: List<T>) : Option<T> = xs | min

[first([1, 2]), smallest([5, 3])] | somes
