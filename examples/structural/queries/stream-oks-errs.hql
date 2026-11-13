// status: valid-now
// feature: `oks()` and `errs()` over a `Stream` of results keep a stream, so `take(n)` bounds it afterwards
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["u-1", "u-3", "u-1"], ["UserData.id_present", "UserData.id_present"]]

type UserData : DataContract = {
    id: String
    require id != "" as id_present
}

// A live source: the three values, over and over, without end.
fn arrivals(xs: List<Data>) : Stream<Data> {
    for x in xs {
        yield x
    }
    for again in arrivals(xs) {
        yield again
    }
}

xs : List<Data> = [{id: "u-1"}, {id: ""}, {id: "u-3"}]
results = arrivals(xs) | validate(UserData)
[
    results | oks() | map(u => u.id) | take(3),
    results | errs() | flat_map(e => e.violations) | map(v => v.rule) | take(2)
]
