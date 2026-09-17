// status: invalid
// feature: incomplete record literal
// implementation: pending
// environment: pure
// error: MissingField
// stage: typecheck
// note: Strict typing rather than duck typing means a record literal is rejected at construction, not when something later reads the missing field. A record with an absent age is not a Person that happens to fail on access; an optional field would have to be declared Option of Int.

type Person = {
    name : String
    age  : Int
}

alice : Person = { name: "Alice" }
alice.name
