// status: invalid
// feature: An implementation's function returns what the trait declares
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `area` of `Shape` returns `Float`. An implementation writing `String` is
// rejected at its result, before any call.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : String = "round"
}

Circle(radius = 1.0f).describe()
