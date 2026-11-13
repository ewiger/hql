// status: valid-now
// feature: A module listing a trait inherits the trait's default body for the function it does not declare
// implementation: implemented
// environment: pure
// modules: traits/lib
// expected-type: List<String>
// expected-json: ["circle with area 3.0", "circle with area 12.0"]

// `shapes.circle` declares `area` and `name`, and `Shape` gives `describe` a
// default body over them. A receiver call and a generic function reach the
// default alike.
import shapes
import shapes.circle

fn describe_all<T : Shape>(xs: List<T>) : List<String> = xs | map(x => x.describe())

[Circle(radius = 1.0f).describe(), describe_all([Circle(radius = 2.0f)])[0].unwrap_or("")]
