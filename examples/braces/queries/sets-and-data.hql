// status: valid-now
// feature: the colon tells braces apart: values make a set, key/value entries make Data
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["0", "3", "{name: \"Alice\"}", "{:}"]

// `{}` is the empty set and `{:}` the empty Data. A declared type gives the
// empty set its element type, as it does any `Set<Never>`.
empty : Set<Int> = {}
primes = {2, 3, 5}
record = {name: "Alice"}
blank : Data = {:}
[fmt("{}", empty | size), fmt("{}", primes | size), fmt("{}", record), fmt("{}", blank)]
