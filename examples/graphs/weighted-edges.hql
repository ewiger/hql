// status: design-question
// feature: graph operation
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Edge]
// expected: works-active and family-claim; missing weights excluded
// note: Uses the explicit weighted projection in fixtures/README.md. Missing weights are excluded, not assumed certain; graph edges preserve relation identity.

knowledge
| graph
| edges
| filter(.weight > 0.5)
