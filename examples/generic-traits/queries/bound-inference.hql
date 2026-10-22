// status: valid-now
// feature: A bound over a generic trait, its open parameter inferred at each call
// implementation: implemented
// environment: pure
// expected-type: List<Option<Int>>
// expected-json: [10, 3, null]

// `E : Index<T>` leaves T to the call: a List<Int> takes Seq's native
// implementation and gives T = Int, a Countdown its own, Index<Int>.
type Countdown { from : Int }

impl Index<Int> for Countdown {
    fn at(self, position: Int) : Option<Int> = match position {
        p given p >= 0 and p <= self.from => some(self.from - p),
        _ => none,
    }
}

fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]
fn fifth_of<T, E : Index<T>>(xs: E) : Option<T> = xs.at(4)

[first_of([10, 20]), first_of(Countdown(from = 3)), fifth_of(Countdown(from = 3))]
