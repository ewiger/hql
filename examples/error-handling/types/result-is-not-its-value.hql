// status: invalid
// feature: A Result is not the value it may hold
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `parse` returns `Result<Int, String>`. The `ok` payload is reached by
// `match`, `map` or `unwrap_or`, never by using the result as an `Int`.
fn parse(s: String) : Result<Int, String> = ok(1)
parse("1") + 1
