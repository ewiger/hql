// status: valid-now
// feature: Two implementations of one trait at different arguments, chosen by the call
// implementation: implemented
// environment: pure
// expected-type: List<Float>
// expected-json: [3.0, 0.75]

// An implementation is its trait, its trait's arguments and its type, so
// Scale<Int> and Scale<Float> for one Vec2 do not overlap. The argument a
// call passes decides which applies.
trait Scale<F> {
    fn scale(self, by: F) : Self
}

type Vec2 { x : Float, y : Float }

impl Scale<Float> for Vec2 {
    fn scale(self, by: Float) : Vec2 = Vec2(x = self.x * by, y = self.y * by)
}

impl Scale<Int> for Vec2 {
    fn scale(self, by: Int) : Vec2 = self.scale(float(by))
}

v = Vec2(x = 1.5f, y = 2.0f)

[v.scale(2).x, v.scale(0.5f).x]
