// status: valid-now
// feature: transport provenance stays outside `Invalid`: an envelope holds it beside the `Invalid`, and a producer's extra field rides on the open record
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["users/7", "UserData", "{id: \"\"}", "\"users.csv\""]

// `Rejected` is a broker's envelope: its `key` is the record's place on the
// broker, and its `value` the rejected record. `Invalid` declares neither.
type Rejected : DataContract = {
    key: String
    value: Invalid
}

received : Data = {
    key: "users/7",
    value: {
        contract: "UserData",
        value: {id: ""},
        violations: [{
            contract: "UserData", path: [], kind: "rule", rule: "UserData.1",
            text: "require id != \"\"", message: "`require id != \"\"` does not hold"
        }],
        source: "users.csv"
    }
}
r = received | Rejected
[r.key, r.value.contract, fmt("{}", r.value.value), fmt("{}", (r.value | Data).source)]
