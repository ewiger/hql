// status: proposed
// feature: table presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: row-and-column presentation of alice, bob, carol
// note: table is a terminal presenter, not a semantic transformation. The collection stays a List[Card]; only the final stage decides how the host shows it. Predicate spelling where(_.type == "person") remains an alternative to filter.
// alternatives: presentation/json.hql, presentation/default.hql

cards
| filter(.type == "person")
| table
