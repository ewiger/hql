// status: valid-now
// feature: A function accepting a wider parameter stands where a narrower one is expected
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [1, 1]

// Parameter positions are contravariant: a function that takes any Data tree
// can be given each Int of a list, so `map` accepts it.
fn one(x: Data) : Int = 1

[10, 20] | map(one)
