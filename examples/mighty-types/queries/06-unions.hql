// status: valid-now
// implementation: implemented
// environment: pure
// feature: A union of record types, matched by type
// expected-type: String
// expected: 3 shapes, total area 19.0

// A union lists its alternatives. A match over it binds each alternative at
// its own type and must cover every member. The annotation on `shapes`
// holds the literal at the union, where the elements alone have no common
// type.
type Circle { radius : Float }
type Square { side : Float }
type Shape = union { Circle, Square }

fn area(s: Shape) : Float = match s {
    Circle c => 3.0 * c.radius * c.radius,
    Square q => q.side * q.side,
}

shapes : List<Shape> = [Circle(radius = 1.0), Square(side = 2.0), Circle(radius = 2.0)]

fmt("{} shapes, total area {}", shapes.size(), shapes | map(area) | sum)
