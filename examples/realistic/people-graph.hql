// status: proposed
// feature: people graph
// implementation: pending
// environment: knowledge-v1
// expected-type: Graph
// expected: people graph with provenance retained
// note: knowledge as a function on selected cards is distinct from the ambient Knowledge value; overload spelling is tentative.

selected =
    cards
    | filter(.type == "person")

selected
| knowledge
| graph
