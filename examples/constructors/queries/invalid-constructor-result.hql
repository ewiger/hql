// status: invalid
// feature: A constructor returns the type it is named after
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `Temp` is a declared type, and a function named `Temp` constructs it: one
// returning an Int is refused.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Int = 1

1
