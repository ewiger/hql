// status: invalid
// feature: A trait naming its type writes Self only as the receiver
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// Two values held at `Figure` may be a `Rect` and a `Circle`, so a function
// relating two values of `Self` has no meaning over it. The refusal names
// `same_size`.
trait Shape as Figure {
    fn area(self) : Float
    fn same_size(self, other: Self) : Bool
}

1
