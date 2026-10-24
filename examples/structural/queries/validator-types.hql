// status: valid-now
// feature: a validator type extends `Validator`; `validate f with V` and a field `f: V` hold where `V`'s `validate` of the field is true
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [["a@b.ch"], ["UserData.work_email: work_email: Email", "UserData.1: validate contact? with Email"]]

// `work_email: Email` is a `String` held to `Email`; `contact?` is held to
// it by a rule, and only where it is present.
trait Email : Validator { }

impl Email for String {
    fn validate(self) : Bool = self.contains("@") and not self.starts_with("@")
}

type UserData : DataContract = {
    contact?: String
    work_email: Email

    validate contact? with Email
}

xs : List<Data> = [
    {work_email: "a@b.ch", contact: "c@d.ch"},
    {work_email: "@b.ch"},
    {work_email: "a@b.ch", contact: "nobody"}
]
results = xs | validate(UserData)
[
    results | oks() | map(u => u.work_email) | List,
    results | errs() | flat_map(e => e.violations) | map(v => fmt("{}: {}", v.rule, v.text)) | List
]
