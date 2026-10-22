// status: invalid
// feature: Two constructors of a type taking the same first parameter are refused
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// The first argument alone selects a constructor, so two forms of one arity
// differ in their first parameter.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0)
fn Temp(fahrenheit: Float) : Temp = Temp(kelvin = fahrenheit)

1
