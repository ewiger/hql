// status: invalid
// feature: A constructor passed as a value where the expected parameter fits several forms is refused
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `[]` holds Never, which every form of `Temp` takes, so `map` does not pick
// one; the refusal names both first parameters, Float and String.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0)
fn Temp(label: String) : Temp = Temp(kelvin = 0.0)

[] | map(Temp)
