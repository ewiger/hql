// status: invalid
// feature: A type parameter without a bound has no functions
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `T` is unbounded, so `x.area()` reaches nothing: inside a generic body
// only what the bound promises is known. `fn widest<T : Shape>` would
// check.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

fn widest<T>(xs: List<T>) : Float = xs | map(x => x.area()) | sum

1
