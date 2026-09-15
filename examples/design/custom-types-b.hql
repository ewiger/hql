// status: design-question
// feature: custom type alternative
// implementation: pending
// environment: knowledge-v1
// expected-type: Int
// expected: 42 after schema validation
// note: Structural intersection alternative; not an implemented grammar.
// alternatives: design/custom-types-a.hql

type Person = Card & { name: String, age: Int }
alice : Person = validate[Person](resolve([[alice]]))
alice.age
