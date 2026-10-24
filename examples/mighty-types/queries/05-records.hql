// status: valid-now
// implementation: implemented
// environment: pure
// feature: Record types, their constructors and field reads
// expected-type: List<Int>
// expected: [12, 4, 3, 1, 1]

// A record declares its shape; its constructor takes the fields by name and
// an incomplete literal is refused. Records nest, and a field read follows
// the path.
type Point { x : Int, y : Int }
type Rectangle { top_left : Point, bottom_right : Point }

fn width(r: Rectangle) : Int = r.bottom_right.x - r.top_left.x
fn height(r: Rectangle) : Int = r.bottom_right.y - r.top_left.y
fn area(r: Rectangle) : Int = width(r) * height(r)

origin = Point(x = 0, y = 0)
r = Rectangle(top_left = origin, bottom_right = Point(x = 4, y = 3))
moved = Point(x = origin.x + 1, y = origin.y + 1)

[area(r), width(r), height(r), moved.x, moved.y]
