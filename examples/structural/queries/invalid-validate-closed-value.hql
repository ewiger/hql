// status: invalid
// feature: `validate(T)` reads a `Data` or an open record; a value already of a closed type is refused
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

type Point = {
    x: Int
}

Point(x = 1) | validate(Point)
