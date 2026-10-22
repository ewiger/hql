// status: invalid
// feature: Every declaration of the program module is public
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// `private` makes a declaration visible within its own module only. No other
// unit reads the program module, so `private` is refused there.
private fn double(n: Int) : Int = n * 2

double(21)
