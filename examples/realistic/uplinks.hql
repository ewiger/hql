// status: proposed
// feature: uplinks
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: bob
// note: Uplinks are the outgoing half of a node edge; downlinks are the incoming half. Authored body links only; a card may be its own uplink and downlink.

resolve([[alice]])
| uplinks
