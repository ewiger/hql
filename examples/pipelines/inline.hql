// status: design-question
// feature: pipeline layout
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: alice, bob, carol
// note: Compare multiline.hql; canonical layout is undecided.
// alternatives: pipelines/multiline.hql

cards | filter(.type == "person") | take(10)
