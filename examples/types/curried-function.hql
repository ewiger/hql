// status: proposed
// feature: curried function type
// implementation: pending
// environment: knowledge-v1
// expected-type: Relation
// expected: relation from alice to bob
// note: Arrows associate right. Candidate constructor produces a Relation Card value, with identity and schema construction policy unresolved.

related : Card -> Card -> Relation = a => b => relation(a, b)
related(alice)(bob)
