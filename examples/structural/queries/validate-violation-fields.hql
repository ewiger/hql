// status: valid-now
// feature: one `Invalid` holds every violation of its value, and a structural `Violation` fills `expected` and `actual`
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["UserData.id.missing", "String", "none"], ["UserData.age.type", "Int", "\"forty-two\""]]

// `{age: "forty-two"}` breaks two fields at once: `id` is missing and `age`
// holds text. Both are in the one `Invalid`. A missing field expects its type
// and holds nothing; a mistyped one holds the tree it has.
type UserData : DataContract = {
    id: String
    age: Int
}

d : Data = {age: "forty-two"}
match d | validate(UserData) {
    ok(_) => [],
    err(e) => e.violations | map(v => [
        v.rule,
        v.expected | unwrap_or("-"),
        fmt("{}", v.actual)
    ]) | List,
}
