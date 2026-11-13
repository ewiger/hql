// status: valid-now
// feature: A generic variadic parameter binds one type from its trailing arguments
// implementation: implemented
// environment: pure
// expected-type: List<List<Int>>
// expected: [[1, 2], [], [5]]

// `xs: ...T` is a List<T>, and the trailing arguments fill `T`, as repeated
// parameters of one type do: `pack(1, 2)` is a `List<Int>`. With no trailing
// argument the list is empty.
fn pack<T>(xs: ...T) : List<T> = xs
fn largest<T : Orderable>(first: T, rest: ...T) : T = rest | max | unwrap_or(first)

[pack(1, 2), pack(), [largest(1, 5, 3)]]
