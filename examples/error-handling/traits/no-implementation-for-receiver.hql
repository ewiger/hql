// status: invalid
// feature: A trait function is a function of the types that implement the trait
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `Line` has no `impl Shape`, so `area` is no function of a `Line`. The
// call resolves by the receiver's type and finds nothing.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

type Line { length : Float }

Line(length = 2.0f).area()
