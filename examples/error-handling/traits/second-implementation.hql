// status: invalid
// feature: A trait has one implementation per type: the checker rejects a second
// implementation: implemented
// environment: pure
// phase: check
// kind: duplicate-implementation
// severity: error

// Coherence: two implementations would make `area` of a `Circle` depend on
// which one a call reached. The second is rejected, naming the first.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f
}

impl Shape for Circle {
    fn area(self) : Float = 4.0f
}

1
