// status: invalid
// feature: An implementation is for a declared type
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `Nope` is no type, so there is nothing to implement `Shape` for.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Nope {
    fn area(self) : Float = 1.0f
}

1
