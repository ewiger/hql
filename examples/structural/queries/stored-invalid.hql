// status: valid-now
// feature: a stored `Invalid` is ordinary `Data`, and `| Invalid` reads it back with its violations
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["UserData UserData.id_present error"]

// The tree is what writing an `Invalid` leaves: `severity` is not stored,
// and reading fills it with its default, "error".
stored : Data = {
    contract: "UserData",
    value: {id: ""},
    violations: [{
        contract: "UserData",
        path: [],
        kind: "rule",
        rule: "UserData.id_present",
        text: "id != \"\"",
        message: "`id != \"\"` (UserData.id_present) does not hold"
    }]
}
e = stored | Invalid
e.violations | map(v => fmt("{} {} {}", e.contract, v.rule, v.severity)) | List
