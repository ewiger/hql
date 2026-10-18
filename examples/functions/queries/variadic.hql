// status: valid-now
// feature: A variadic parameter collects the trailing arguments of a call
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [0, 3, 10]

// `xs: ...Int` is a List<Int> filled from zero or more trailing arguments.
fn total(xs: ...Int) : Int = xs | sum

[total(), total(1, 2), total(1, 2, 3, 4)]
