// status: invalid
// feature: An execute failure inside called functions carries the call frames that reached it and the bindings in scope where it failed
// implementation: implemented
// environment: pure
// expected-type: Int
// phase: execute
// kind: division-by-zero
// severity: failure

// `share` calls `ratio` with `parts` 0. The diagnostic's `trace` holds the
// two `call` frames, outermost first, and its `bindings` the parameters of
// `ratio`, the function the division is in.
fn ratio(total: Int, parts: Int) : Int = total / parts

fn share(xs: List<Int>) : Int = ratio(xs | sum, size(xs) - 3)

share([4, 5, 6])
