// status: valid-now
// feature: `{ a, b } = xs | fork { … }` binds each branch; `parts.a` reads one from the record
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["ada", "grace"], ["linus"], ["ada", "grace"]]

type Person {
    name : String
    born : Int
}

people = [Person(name = "ada", born = 1815), Person(name = "linus", born = 1969), Person(name = "grace", born = 1906)]

{ early, late } =
    people
    | fork {
        early = filter(p => p.born < 1950) | map(p => p.name)
        late = filter(p => p.born >= 1950) | map(p => p.name)
    }

parts = people | fork { early = filter(p => p.born < 1950) | map(p => p.name) }
[early, late, parts.early]
