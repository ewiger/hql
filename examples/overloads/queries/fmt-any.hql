// status: valid-now
// feature: A variadic parameter declared ...Any takes a value of any type in each position
// implementation: implemented
// environment: pure
// expected-type: String
// expected-json: "1 of [2, 3], some 4"

// `fmt(self, args: ...Any)`: each argument after the template has its own
// type, where `...T` would bind one type for them all.
fmt("{} of {}, some {}", 1, [2, 3], some(4))
