// status: invalid
// feature: A call fills a bounded parameter with a type that implements the trait
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `total` needs `T : Shape`, and `Line` implements nothing. The call is
// rejected where it is written, naming the bound and the type.
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

type Line { length : Float }

fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum

total([Line(length = 2.0f)])
