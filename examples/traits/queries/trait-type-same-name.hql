// status: proposed
// feature: A trait names its type with its own name, read as the trait in a bound and as the type elsewhere
// implementation: pending
// proposal: doc/proposals/HQL-0039/README.md
// environment: pure
// expected-type: List<String>
// expected-json: ["rex", "rock of 3"]

trait Named as Named {
    fn name(self) : String
}

type Dog { tag : String }
type Rock { weight : Int }

impl Named for Dog {
    fn name(self) : String = self.tag
}

impl Named for Rock {
    fn name(self) : String = fmt("rock of {}", self.weight)
}

fn names<T : Named>(xs: List<T>) : List<String> = xs | map(x => x.name())

things : List<Named> = [Dog(tag = "rex"), Rock(weight = 3)]

names(things)
