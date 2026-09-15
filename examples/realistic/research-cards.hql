// status: proposed
// feature: recent research
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: research/type-notes, research/graph-notes, carol
// note: Fixture updated values are ISO date strings. desc syntax and timestamp types remain proposed.

cards
| filter(.tags contains "research")
| sort(.updated desc)
| take(10)
