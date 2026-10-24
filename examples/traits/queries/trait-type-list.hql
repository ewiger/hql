// status: proposed
// feature: A trait names the union of its implementers as a type, and a list holds values of every member
// implementation: pending
// proposal: doc/proposals/HQL-0039/README.md
// environment: pure
// expected-type: List<String>
// expected-json: ["area 6.0", "area 3.0"]

trait Shape as Figure {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Rect { width : Float, height : Float }
type Circle { radius : Float }

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

figures : List<Figure> = [Rect(width = 2.0f, height = 3.0f), Circle(radius = 1.0f)]

figures | map(f => f.describe())
