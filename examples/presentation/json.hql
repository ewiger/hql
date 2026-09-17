// status: proposed
// feature: json presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: structured serialization of the same three person cards
// note: Same value as the table case, different view. A debug or structured presenter needs a serialization contract for Card, Frontmatter and Hmd that the corpus does not yet fix.
// alternatives: presentation/table.hql

cards
| filter(.type == "person")
| json
