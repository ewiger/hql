// status: valid-now
// feature: A receiver module lists a trait, and its self functions are the implementation, reached through a bound and as a receiver call
// implementation: implemented
// environment: pure
// modules: traits/lib
// expected-type: List<Float>
// expected-json: [12.0, 3.0, 3.0, 2.0]

// `module shapes.circle for Circle : Shape` declares the one implementation
// of `Shape` for `Circle`: its `area` and `name` are the trait's. `c.area()`,
// `circle.area(c)` and `shapes.total` through its bound `T : Shape` reach the
// one declaration; `diameter` is listed in no trait and stays a receiver
// function.
import shapes
import shapes.circle

c = Circle(radius = 1.0f)

[shapes.total([c, Circle(radius = 1.0f), c, c]), c.area(), circle.area(c), c.diameter()]
