// status: valid-now
// feature: rejected values written by `errs() | sink(…)` read back as `Invalid`, and a repair from `e.value` runs through another validation and `fork`
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: List<List<String>>
// expected: [["u-new Nobody", "u-new Anon"], ["UserData.id_present", "UserData.id.missing"]]

// `rejected.jsonl` is what `tap(…) | validate(UserData) | errs() | sink(…)`
// wrote of users.jsonl: one `Invalid` per rejected value, with its `value`.
type UserData : DataContract = {
    id: String
    name: String
    require id != "" as id_present
}

fn repair(d: Data) : Data = merge(d, {id: "u-new"})

stored = source(file("../data/rejected.jsonl"), JsonLines) | Invalid
{ repaired, still_invalid } =
    stored
    | map(e => repair(e.value))
    | validate(UserData)
    | fork {
        repaired = oks()
        still_invalid = errs()
    }

[
    repaired | map(u => fmt("{} {}", u.id, u.name)) | List,
    stored | flat_map(e => e.violations) | map(v => v.rule) | List
]
