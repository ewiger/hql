// status: design-question
// feature: pure transformation
// implementation: pending
// environment: knowledge-v1
// expected-type: Card
// expected: modified Card value; stored source remains unchanged
// note: set constructs a value. Future save : Card -> IO[Card] would be explicit and effectful; embedded eval is intended to start pure.

alice = resolve([[alice]])
updated = alice | set .fm.status "reviewed"
updated
