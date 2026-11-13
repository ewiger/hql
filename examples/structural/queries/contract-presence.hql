// status: valid-now
// feature: `f?` standing as a condition is the presence of `f`, and `=>` in a rule is implication
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["open", "closed"]

// Each account gives an email or a phone, and a closed one says when it closed.
type Account : DataContract = {
    status: String
    closed_at?: String
    email?: String
    phone?: String

    require email? or phone?
    require status == "closed" => closed_at?
}

xs : List<Data> = [
    {status: "open", email: "a@example.com"},
    {status: "closed", phone: "+41", closed_at: "2026-10-01"}
]
xs | Account | map(a => a.status) | List
