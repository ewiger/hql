// status: valid-now
// feature: A step over a value, applied to absence, evaluates to absence
// implementation: implemented
// environment: pure
// expected-type: List<Option<Int>>
// expected-json: [2, null, null, 3]

// `abs` takes an Int, not an Option<Int>. Over an optional input the step is
// lifted: a present value is taken and its result is present, and absence
// gives absence at the step's result type, here Option<Int>.
[[some(-2), none] | map(x => x | abs), [none | abs, some("abc") | length]] | emit
