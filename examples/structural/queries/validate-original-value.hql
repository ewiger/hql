// status: valid-now
// feature: `Invalid.value` is the `Data` that was validated, as it was before any default was applied
// implementation: implemented
// environment: pure
// expected-type: List<Data>
// expected: [{id: "", note: "blank"}, {name: "Bob"}]

// `age` has a default, and neither rejected value has an `age`: the value
// kept is the one read, not one with the default filled in.
type UserData : DataContract = {
    id: String
    age: Int = 18
    require id != ""
}

xs : List<Data> = [{id: "u-1"}, {id: "", note: "blank"}, {name: "Bob"}]
xs | validate(UserData) | errs() | map(e => e.value) | List
