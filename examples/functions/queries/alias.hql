// status: valid-now
// feature: An alias is a function whose types the checker writes
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 2]

// `top(n) := sort | take(n)` opens with a step, so its input is implicit and
// takes the most general type the body accepts: a Collection of orderable
// elements. `n` is an Int because `take` requires one. `hql check` prints
// `top : (Collection<T>, Int) -> List<T> where T : Orderable`.
top(n) := sort | take(n)

[3, 1, 2] | top(2)
