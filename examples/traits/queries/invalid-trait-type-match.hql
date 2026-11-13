// status: invalid
// feature: A match over a trait's named type covers every implementing type
// implementation: implemented
// environment: pure
// phase: check
// kind: non-exhaustive-match
// severity: error

// `Figure`'s members are `Rect` and `Circle`, the types implementing
// `Shape`. The match covers `Rect` alone and has no `_` arm.
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
}

label(Rect(width = 2.0f, height = 3.0f))
