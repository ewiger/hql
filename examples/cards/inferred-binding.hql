// status: proposed
// feature: inferred binding
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: carol, family-record, research/graph-notes
// note: Preferred inferred binding, alice : Card. Resolution is an ordinary typed function. Downlinks here count authored body links only.

alice = resolve([[alice]])

alice
| downlinks
