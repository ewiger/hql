// status: proposed
// feature: downlinks
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: carol, family-record, research/graph-notes
// note: Downlinks are the incoming half of a node edge; uplinks are the outgoing half. Incoming authored links only in this fixture profile; declarations are a different evidence origin.

resolve([[alice]])
| downlinks
