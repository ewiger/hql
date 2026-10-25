// status: invalid
// feature: Text that is not JSON is an evaluation failure of the parse
// implementation: implemented
// environment: pure
// expected-type: Data
// error: RuntimeError
// stage: evaluate

// `Data(s, Json)` is `String -> Data`, which checks for any string. That
// this string is not JSON is a fact about the value.
Data("{", Json)
