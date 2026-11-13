// status: invalid
// feature: A match over a union covers every member, or the checker rejects it
// implementation: implemented
// environment: pure
// phase: check
// kind: non-exhaustive-match
// severity: error

// `Shape` has two members and the match names one. The value passed is a
// `Circle`, which the missing arm would never have been reached for; the
// checker rejects the match all the same, since it covers the type, not
// this value.
type Circle { r : Int }
type Square { s : Int }
type Shape = union { Circle, Square }

fn area(s: Shape) : Int = match s {
    Circle c => c.r,
}

area(Circle(r = 1))
