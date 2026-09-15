// status: design-question
// feature: graph operation
// implementation: pending
// environment: knowledge-v1
// expected-type: Graph
// expected: induced graph on alice, bob, carol
// note: Graph is a value. Direction, edge identity, weight projection and ordering need explicit contracts.

knowledge
| graph
| subgraph(nodes = people)
