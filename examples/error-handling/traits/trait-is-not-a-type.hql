// status: invalid
// feature: A trait is a contract, not a type: no parameter is typed by one
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-name
// severity: error

// `Shape` is a trait. It names a type nowhere: not in a parameter, a
// binding or a list. A function over "any Shape" is written with a bound,
// `fn doubled<T : Shape>(s: T)`, and a value never carries a trait.
trait Shape {
    fn area(self) : Float
}

fn doubled(s: Shape) : Float = s.area() * 2.0f

1
