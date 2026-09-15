// status: design-question
// feature: pipeline layout
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: alice, bob, carol
// note: Leading pipes make stages visible; line continuation rules remain open.
// alternatives: pipelines/inline.hql

cards
| filter(.type == "person")
| take(10)
