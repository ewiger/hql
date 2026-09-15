// status: design-question
// feature: graph operation
// implementation: pending
// environment: knowledge-v1
// expected-type: Option<List[Card]>
// expected: candidate directed path alice -> bob -> carol
// note: Graph is a value. Direction, edge identity, weight projection and ordering need explicit contracts.

knowledge
| graph
| path([[alice]], [[carol]])
