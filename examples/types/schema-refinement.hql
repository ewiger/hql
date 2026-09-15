// status: design-question
// feature: custom schema validation
// implementation: pending
// environment: knowledge-v1
// expected-type: Int
// expected: 42
// note: Person schema is in fixtures/README.md. Successful validation strengthens age; failure carrier and refinement spelling remain open.

person : Person = validate[Person](resolve([[alice]]))
person.age
