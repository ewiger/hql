// status: valid-now
// feature: a type narrowing Data is an open view: it reads its fields and keeps the rest
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["Alice", "+41"]

// `User : Data` declares the part of the tree the program relies on; `phone`
// is not part of it, and is kept beneath the view. Reading the same value as
// `Contact` sees it: typing changes what is known of a value, not the value.
type User : Data = {
    id: String
    name: String
}

type Contact : Data = {
    phone: String
}

d : Data = {id: "42", name: "Alice", phone: "+41", country: "CH"}
user = d | User
[user.name, (user | Contact).phone]
