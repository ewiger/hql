// status: invalid
// feature: Text that is not JSON is an evaluation failure of the parse
// implementation: implemented
// environment: pure
// expected-type: Data
// phase: execute
// kind: malformed-data
// severity: failure

// `Data(s, Json)` is `String -> Data`, which checks for any string. That
// this string is not JSON is a fact about the value.
Data("{", Json)
