// status: valid-now
// feature: The same data failure read as a value with validate: no report, exit 0
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["ada is 36","rejected: User"]

// `validate(User)` gives `Result<User, Invalid>`, so the record with
// `age: "forty"` is an `err` the program matches on. The run has nothing to
// report: handling a failure and reporting one are different channels.
type User { name : String, age : Int }

users = [{name: "ada", age: 36}, {name: "bob", age: "forty"}]

users | map(u => match u | validate(User) {
    ok(v) => fmt("{} is {}", v.name, v.age),
    err(e) => fmt("rejected: {}", e.contract),
})
