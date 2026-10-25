// status: invalid
// feature: A constructor returns the type it is named after
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `Temp` is a declared type, and a function named `Temp` constructs it: one
// returning an Int is refused.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Int = 1

1
