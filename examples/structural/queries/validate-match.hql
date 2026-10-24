// status: valid-now
// feature: one value validated and handled in a `match`, with no exception
// implementation: implemented
// environment: pure
// expected-type: String
// expected: 1 violation of UserData

type UserData : DataContract = {
    id: String
    require id != ""
}

d : Data = {id: ""}
match d | validate(UserData) {
    ok(user) => user.id,
    err(e) => fmt("{} violation of {}", e.violations.size(), e.contract),
}
