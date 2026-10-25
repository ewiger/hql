// status: invalid
// feature: Implementing a trait that extends another does not implement the other
// implementation: implemented
// environment: pure
// phase: check
// kind: unsatisfied-bound
// severity: error

// `Greeter : Named` makes `name` a function of `Greeter`, and `impl Greeter
// for Dog` supplies it. A bound `T : Named` still takes only a type with an
// `impl Named` of its own.
trait Named {
    fn name(self) : String
}

trait Greeter : Named { }

type Dog { tag : String }

impl Greeter for Dog {
    fn name(self) : String = self.tag
}

fn names<T : Named>(xs: List<T>) : List<String> = xs | map(x => x.name())

names([Dog(tag = "rex")])
