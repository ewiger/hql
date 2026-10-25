// status: invalid
// feature: An implementation of a core trait takes the type the trait declares
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `add` of `Add` takes a value of the implementing type. An implementation
// taking `Int` is rejected at that parameter: `Vec2 + 3` does not become
// well typed by writing an `impl` that would accept it.
type Vec2 { x : Int, y : Int }

impl Add for Vec2 {
    fn add(self, other: Int) : Vec2 = Vec2(x = self.x + other, y = self.y)
}

Vec2(x = 1, y = 2) + 3
