// status: invalid
// feature: A call no form of an overloaded operation takes is refused naming every form
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `pow` is declared over Ints and over Floats, each exponent of its base's
// type. An Int raised to a Float fits neither, and the refusal names both
// forms: nothing widens on its own, so write `float(2) | pow(2.0)`.
2 | pow(2.0)
