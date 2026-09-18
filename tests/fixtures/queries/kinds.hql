// What the vault holds, by kind.
//
// Run it:  hql --vault tests/fixtures/vault run tests/fixtures/queries/kinds.hql
//
// A card declaring `knowledge.type: Relation` is only a RelationCard once its
// endpoints resolve. `broken-relation` declares it and does not qualify, so it
// appears here as a ConceptCard, with its failed claim kept in its metadata
// rather than discarded.

cards
| sort
| table
