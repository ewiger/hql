// status: invalid
// feature: An implementation's function takes the parameters the trait declares
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `area` takes nothing after `self`. An implementation adding `scale` is
// not an `area`, and the `impl` is rejected.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self, scale: Float) : Float = scale
}

Circle(radius = 1.0f).describe()
