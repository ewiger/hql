// status: valid-now
// feature: A program declares constructors for its own type, told apart by their first parameter
// implementation: implemented
// environment: pure
// expected-type: List<Float>
// expected-json: [274.0, 0.0, 5.0, 275.0]

// A function named after a type the program declares, returning that type,
// is a constructor of it. `Temp` has two, one taking a Float and one a
// String, and the first argument selects one, called, piped or passed to
// `map`. Fields given by name build the record itself.
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0f)
fn Temp(label: String) : Temp = Temp(kelvin = 0.0f)

[
    [Temp(1.0f).kelvin, ("absolute zero" | Temp).kelvin, Temp(kelvin = 5.0f).kelvin],
    [2.0f] | map(Temp) | map(t => t.kelvin)
] | emit
