// status: proposed
// feature: currying
// implementation: pending
// environment: knowledge-v1
// expected-type: Relation
// expected: relation from alice to bob
// note: Candidate relation constructor returns an unpersisted Relation Card value; identity/default fields need design. Curried application consumes one Card at each step.

related = a => b => relation(a, b)
relatedToAlice = related(alice)
relatedToAlice(bob)
