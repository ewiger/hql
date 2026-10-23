// status: valid-now
// feature: `fork { accepted = oks(), rejected = errs() }` over a `List`, `Seq` and `Stream` of results: each value lands on one side, each side in source order
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["u-1", "u-3"], ["UserData", "UserData"], ["u-1", "u-3"], ["UserData", "UserData"], ["u-1", "u-3", "u-1"], ["UserData", "UserData", "UserData"]]

// The same four values as a list, a sequence that ends and a stream that
// does not. There is no operation that routes results; `fork` broadcasts
// them and `oks()` and `errs()` keep complementary sides.
type UserData : DataContract = {
    id: String
    require id != "" as id_present
}
fn arrivals(xs: List<Data>) : Stream<Data> {
    for x in xs {
        yield x
    }
    for again in arrivals(xs) {
        yield again
    }
}
fn arrived(xs: List<Data>) : Seq<Data> {
    for x in xs {
        yield x
    }
}
xs : List<Data> = [{id: "u-1"}, {id: ""}, {id: "u-3"}, {id: ""}]
{ accepted, rejected } = xs | validate(UserData) | fork { accepted = oks(), rejected = errs() }
s = arrived(xs) | validate(UserData) | fork { accepted = oks(), rejected = errs() }
live = arrivals(xs) | validate(UserData) | fork { accepted = oks(), rejected = errs() }
[
    accepted | map(u => u.id),
    rejected | map(e => e.contract),
    s.accepted | map(u => u.id) | List,
    s.rejected | map(e => e.contract) | List,
    live.accepted | map(u => u.id) | take(3),
    live.rejected | map(e => e.contract) | take(3)
]
