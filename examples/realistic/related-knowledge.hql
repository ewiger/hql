// status: proposed
// feature: related evidence graph
// implementation: pending
// environment: knowledge-v1
// expected-type: Graph
// expected: alice-centered evidence graph, preserving provenance

focus = [[alice]]
knowledge
| about(focus)
| graph
