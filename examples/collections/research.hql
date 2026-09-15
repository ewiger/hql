// status: proposed
// feature: contains and sort
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: carol, research/graph-notes, research/type-notes
// note: Card tags/type shortcuts and infix contains are candidate sugar over metadata.

cards
| filter(.tags contains "research")
| sort(.title)
