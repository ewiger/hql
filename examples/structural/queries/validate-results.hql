// status: valid-now
// feature: `x | validate(T)` gives a `Result<T, Invalid>` per value, and `oks` and `errs` keep the successes and the failures
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["u-1", "u-4"], ["UserData.id_present", "UserData.2", "UserData.id.missing", "UserData.age.type"]]

// The third value is missing `id` and has a `String` age: both are
// reported, with no rule run. The fourth breaks only a warning and reads.
type UserData : DataContract = {
    id: String
    age?: Int

    require id != "" as id_present
    require age? >= 0
    warn age? < 120
}

xs : List<Data> = [{id: "u-1", age: 34}, {id: "", age: -3}, {age: "x"}, {id: "u-4", age: 130}]
results = xs | validate(UserData)
[
    results | oks() | map(u => u.id) | List,
    results | errs() | map(e => e.violations) | emit() | map(v => v.rule) | List
]
