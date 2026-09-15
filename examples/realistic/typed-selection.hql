// status: design-question
// feature: typed selection
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Person]
// expected: alice, carol
// note: Preferred selection style refines matching Cards using Person schema, rather than trusting fm.type alone. Invalid matching records need a diagnostic policy.

cards
| typed Person
| filter(.age >= 18)
