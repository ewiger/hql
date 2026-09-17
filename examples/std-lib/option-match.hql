// status: proposed
// feature: exhaustive match on Option
// implementation: pending
// environment: knowledge-v1
// expected-type: String
// expected: Alice
// note: Alice declares no nickname, so the optional schema field is None rather than a missing key. The match must cover both variants, and that requirement is what replaces the null check: dropping the None arm is a type error rather than a fault that waits for the wrong card.

person : Person = validate<Person>(alice)

match person.nickname {
    Some(n) => n
    None    => person.name
}
