// status: proposed
// feature: A trait's named type satisfies a bound on the trait
// implementation: pending
// proposal: doc/proposals/HQL-0039/README.md
// environment: pure
// expected-type: Float
// expected: 9.0

trait Shape as Figure {
    fn area(self) : Float
}

type Rect { width : Float, height : Float }
type Circle { radius : Float }

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum

figures : List<Figure> = [Rect(width = 2.0f, height = 3.0f), Circle(radius = 1.0f)]

total(figures)
