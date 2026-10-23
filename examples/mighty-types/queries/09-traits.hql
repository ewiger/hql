// status: valid-now
// implementation: implemented
// environment: pure
// feature: A trait, two implementations, a default body and a bounded generic
// expected-type: List<String>
// expected-json: ["circle with area 3.0","15.0","6.0","circle with area 12.0"]

// A trait is a compile-time contract. `impl Shape for Circle` provides it,
// `describe` has a default body every implementation inherits, and
// `T : Shape` lets a generic function call the trait's functions on `T`.
trait Shape {
    fn area(self) : Float
    fn name(self) : String
    fn describe(self) : String = fmt("{} with area {}", self.name(), self.area())
}

type Circle { radius : Float }
type Rect { width : Float, height : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
    fn name(self) : String = "circle"
}

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
    fn name(self) : String = "rect"
}

fn total_area<T : Shape>(shapes: List<T>) : Float = shapes | map(s => s.area()) | sum
fn largest<T : Shape>(shapes: List<T>) : Option<String> =
    shapes | sort(by = s => s.area()) | last | map(s => s.describe())

circles = [Circle(radius = 1.0f), Circle(radius = 2.0f)]
rects = [Rect(width = 2.0f, height = 3.0f)]

[
    Circle(radius = 1.0f).describe(),
    str(total_area(circles)),
    str(total_area(rects)),
    largest(circles).unwrap_or("none"),
]
