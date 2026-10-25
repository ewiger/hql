// status: invalid
// feature: A binding's annotation is checked against its value
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `n : Int` states the type, and `"one"` is a `String`. The checker rejects
// the binding; nothing is coerced at run time.
n : Int = "one"
n
