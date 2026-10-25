// status: invalid
// feature: A trait's function is called on a value, not as a free function
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `area` takes `self`, so it is reached as `circle.area()`. No binding
// named `area` exists for `area(circle)` to reach.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f
}

area(Circle(radius = 1.0f))
