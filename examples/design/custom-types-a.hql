// status: design-question
// feature: custom type alternative
// implementation: pending
// environment: knowledge-v1
// expected-type: Int
// expected: 42 after schema validation
// note: Subtype declaration candidate; required schema fields are specified in fixture prose, not this incomplete syntax.
// alternatives: design/custom-types-b.hql

Person <: Card
alice : Person = validate[Person](resolve([[alice]]))
alice.age
