// status: invalid
// feature: The body of an implementation's function is type-checked like any body
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `self.area()` is a `Float` and `1` an `Int`. The body of `describe` is
// rejected inside the `impl`, with the trait's signature in force.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = self.radius
    fn describe(self) : String = self.area() + 1
}

1
