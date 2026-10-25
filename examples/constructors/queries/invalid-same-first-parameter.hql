// status: invalid
// feature: Two constructors of a type taking the same first parameter are refused
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// The first argument alone selects a constructor, so two forms of one arity
// differ in their first parameter.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0f)
fn Temp(fahrenheit: Float) : Temp = Temp(kelvin = fahrenheit)

1
