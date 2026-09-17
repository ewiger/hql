// status: proposed
// feature: record type
// implementation: pending
// environment: pure
// expected-type: Int
// expected: 42
// note: A record declares a shape, so field access is checked against the declaration rather than attempted against the value. This is the deliberate opposite of the Data tree, where the shape is carried by the value; see std-lib/refinement-structural.hql for how a tree earns a record's guarantees.

type Person = {
    name : String
    age  : Int
}

alice : Person = { name: "Alice", age: 42 }
alice.age
