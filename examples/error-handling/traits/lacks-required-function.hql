// status: invalid
// feature: An implementation supplies every required function, or the checker rejects it at the impl
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `Shape` requires `area`; `describe` has a default. This `impl` writes
// `describe` alone, and the diagnostic names what it lacks. The program
// calls only `describe`, which exists; it is rejected all the same, since a
// contract is verified whole.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn describe(self) : String = "a circle"
}

Circle(radius = 1.0f).describe()
