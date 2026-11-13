// status: invalid
// feature: A trait's named type implements the trait through its members and takes no impl of its own
// implementation: implemented
// environment: pure
// phase: check
// kind: duplicate-implementation
// severity: error

// `Figure` is the union of the types implementing `Shape`, and a call
// through it dispatches to each member's implementation. An `impl` for
// `Figure` would be a second implementation beside that one.
trait Shape as Figure {
    fn area(self) : Float
}

type Rect { width : Float, height : Float }

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}

impl Shape for Figure {
    fn area(self) : Float = 0.0f
}

1
