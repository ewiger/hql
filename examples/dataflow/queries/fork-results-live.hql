// status: valid-now
// feature: a live source validated and forked into accepted and rejected is read once: the rejected side, read after the file is replaced, still sees the first read
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: List<List<String>>
// expected: [["Ada", "Grace"], ["UserData.id_present", "UserData.id.missing"]]

type UserData : DataContract = {
    id: String
    name: String
    require id != "" as id_present
}

{ accepted, rejected } =
    tap(file("../data/users.jsonl"), JsonLines)
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }

names = accepted | map(u => u.name) | take(9)
replaced = [{id: "u-9", name: "Later"}] | target(file("../data/users.jsonl"), JsonLines)
[names, rejected | flat_map(e => e.violations) | map(v => v.rule) | take(9)]
