// status: valid-now
// feature: a plain structural type is closed: exactly its declared fields
// implementation: implemented
// environment: pure
// expected-type: Point
// expected: Point(x = 1.0, y = 2.0)

type Point = {
    x: Float
    y: Float
}

d : Data = {x: 1.0, y: 2.0}
d | Point
