// status: design-question
// feature: declaration alternative
// implementation: pending
// environment: knowledge-v1
// expected-type: Unit or Relation
// expected: unresolved: assertion effect versus constructed relation
// note: Historical alternative. Prefer Relation Cards for significant relationships; pure relation construction must be distinct from assertion effects.
// alternatives: design/declaration-arrow.hql, declarations/assertion.hql

alice
| relatedTo(bob, kind = daughter)
