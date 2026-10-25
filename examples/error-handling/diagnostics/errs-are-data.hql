// status: valid-now
// feature: validate keeps a failure as data: errs() reads the Invalid values and the run writes no diagnostic
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["User"]

// `validate(User)` gives `Result<User, Invalid>`, and `errs()` keeps the
// `err` values. Nothing here is a diagnostic: the run's channel stays empty
// and the exit status is `0`.
type User { name : String, age : Int }

[{name: "ada", age: 36}, {name: "bob", age: "forty"}]
    | map(u => u | validate(User))
    | errs()
    | map(e => e.contract)
