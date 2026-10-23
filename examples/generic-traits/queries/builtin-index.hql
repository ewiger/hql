// status: valid-now
// feature: A generic built-in type implements Index through a native header
// implementation: implemented
// environment: pure
// expected-type: List<Option<Int>>
// expected-json: [20, 7, null]

// `std/core/mod.hql` declares `impl<T> Index<T> for Seq<T>`: a List is a Seq, so
// it takes the trait's function `at` and satisfies `E : Index<T>`.
fn last_of<T, E : Index<T>>(xs: E, length: Int) : Option<T> = xs[length - 1]

[[10, 20, 30].at(1), last_of([5, 6, 7], 3), last_of([1], 2)]
