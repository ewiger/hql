// status: proposed
// feature: backlinks
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: carol, family-record, research/graph-notes
// note: Incoming authored links only in this fixture profile; declarations are a different evidence origin.

resolve([[alice]])
| backlinks
