// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Absence as a value: optional fields, lookups and the Option combinators
// expected-type: List<String>
// expected-json: ["ada","nobody","example.org","no email","ada@example.org,-","1"]

// An optional field reads as `Option<String>`, a position past the end is
// absence, and every way out of an Option says what absence means. `map`,
// `and_then`, `unwrap_or` and `somes` are receiver functions of the
// prelude's `option` module.
type User { name : String, email? : String }

users = [User(name = "ada", email = "ada@example.org"), User(name = "bob")]

fn domain(u: User) : Option<String> = u.email.and_then(e => e.split("@").last())

[
    users.first().map(u => u.name).unwrap_or("nobody"),
    users[5].map(u => u.name).unwrap_or("nobody"),
    domain(users[0].unwrap_or(User(name = "x"))).unwrap_or("?"),
    match users.last() {
        some(u) given u.email.is_some() => "has email",
        some(_) => "no email",
        none => "empty",
    },
    users.map(u => u.email.unwrap_or("-")).join(","),
    str(users.map(domain).somes().size()),
]
