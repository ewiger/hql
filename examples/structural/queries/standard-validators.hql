// status: valid-now
// feature: `import validators` gives the standard validator types: `Email` for a field, and `NonEmpty`, implemented for `String` and `List`, in a rule
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["ada@lovelace.org", "Signup.email Signup.1 Signup.tagged", "Signup.email"]

// The second value breaks three checks: an address with no domain, a name
// of whitespace and no tags. `as tagged` names the rule over `tags`; the
// field declared `email: Email` is reported as `Signup.email`.
import validators

type Signup : DataContract = {
    email: Email
    name: String
    tags: List<String>

    validate name with NonEmpty
    validate tags with NonEmpty as tagged
}

xs : List<Data> = [
    {email: "ada@lovelace.org", name: "Ada", tags: ["math"]},
    {email: "ada@", name: "  ", tags: []},
    {email: "a@b@c.org", name: "Bob", tags: ["x"]}
]
xs | validate(Signup) | map(r => match r {
    ok(s) => s.email,
    err(e) => e.violations | map(v => v.rule) | join(" "),
}) | List
