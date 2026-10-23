// status: valid-now
// feature: `map`, `map_err` and `match` over each `Result` of a `List`, a `Seq` and a `Stream`
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["u-1", "not UserData", "u-3"], ["u-1", "-", "u-3"], ["u-1", "not UserData", "u-3", "u-1"]]

// The outer `map` is over the collection's elements; the inner `map` or
// `map_err` is over one side of one `Result`.
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
fn arrived() : Seq<Data> {
    for x in xs {
        yield x
    }
}

fn said(r: Result<String, String>) : String = match r {
    ok(id) => id,
    err(contract) => fmt("not {}", contract),
}

[
    xs | validate(UserData) | map(r => r.map(u => u.id).map_err(e => e.contract)) | map(said) | List,
    arrived() | validate(UserData) | map(r => match r { ok(u) => u.id, err(e) => "-" }) | List,
    arrivals(xs) | validate(UserData) | map(r => r.map_err(e => e.contract).map(u => u.id)) | map(said) | take(4)
]
