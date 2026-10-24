// status: valid-now
// feature: A receiver function, a trait default body and an impl function, each with a block body
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["area 6.0", "6"]

// Wherever a function is declared, its body may be a block: in a trait, as
// a default body; in an `impl`; and in a receiver module, `module m for T`.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String {
        a = self.area()
        fmt("area {}", a)
    }
}

type Rect { width : Float, height : Float }

impl Shape for Rect {
    fn area(self) : Float {
        w = self.width
        h = self.height
        w * h
    }
}

module counting for Int

fn twice(self) : Int {
    doubled = self * 2
    doubled
}

[Rect(width = 2.0f, height = 3.0f).describe(), str(3.twice())]
