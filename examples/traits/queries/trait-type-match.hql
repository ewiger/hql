// status: proposed
// feature: A match over a trait's named type covers each implementing type
// implementation: pending
// proposal: doc/proposals/HQL-0039/README.md
// environment: pure
// expected-type: List<String>
// expected-json: ["rect 2.0 by 3.0", "circle of radius 1.0"]

trait Shape as Figure {
    fn area(self) : Float
}

type Rect { width : Float, height : Float }
type Circle { radius : Float }

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

fn label(f: Figure) : String = match f {
    Rect r => fmt("rect {} by {}", r.width, r.height),
    Circle c => fmt("circle of radius {}", c.radius),
}

[label(Rect(width = 2.0f, height = 3.0f)), label(Circle(radius = 1.0f))]
