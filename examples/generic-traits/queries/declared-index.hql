// status: valid-now
// feature: A generic declared type implements Index<T> and takes xs[i]
// implementation: implemented
// environment: pure
// expected-type: List<Option<String>>
// expected-json: ["raven", "robin", "wren"]

// `impl<T>` binds the element type of the implementing type, and the trait's
// argument names it: a Ring of String positions are Strings. `xs[i]` over a
// type implementing Index means `xs.at(i)`.
type Ring<T> { items : List<T> }

impl<T> Index<T> for Ring<T> {
    fn at(self, position: Int) : Option<T> = self.items[position % size(self.items)]
}

birds = Ring(items = ["raven", "robin", "wren"])

[birds[0], birds[4], birds.at(5)]
