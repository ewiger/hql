// status: invalid
// feature: A field the record does not declare is a name error
// implementation: implemented
// environment: pure
// phase: check
// kind: unknown-field
// severity: error

// `User` declares `name` alone. Reading `email` is rejected at the read: a
// record's fields are known from its declaration, not from a value.
type User { name : String }
User(name = "ada").email
