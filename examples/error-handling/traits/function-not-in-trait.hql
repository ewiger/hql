// status: invalid
// feature: An implementation writes only the functions its trait declares
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `perimeter` is no function of `Shape`. An `impl` block is the trait's
// contract and nothing more; a function of the type's own belongs in a
// receiver module.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f
    fn perimeter(self) : Float = 6.0f
}

Circle(radius = 1.0f).describe()
