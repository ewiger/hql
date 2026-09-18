# `Card` is a base representation with two kinds

Recorded 2026-09-18 from a user correction, replacing the earlier
`cards-as-concept-nodes` note.

- `Card <: Doc` is the **base representation type** of the knowledge domain: a
  document that gives a piece of knowledge a physical form. It says nothing
  about what the card is *about*. The earlier `Card <: Concept` is withdrawn —
  that parent belongs to one of the two kinds, not to the base.
- `ConceptCard <: {Card, Concept}` is the ordinary case and the default. It
  occupies a node position in a knowledge graph. The supertype set normalizes:
  `{Card, Concept, Doc}` names `Doc` twice, since `Card <: Doc` already holds,
  so the form written is `{Card, Concept}`. The user's phrasing was
  `{Node, Doc}`; `Concept` is the spelling, because there is no `Node` type and
  `Concept` is what fills the node position in `Graph[Concept, Relation]`.
- `RelationCard <: Card` is the exception, declared by
  `card.metadata.knowledge.type == Relation`. Most knowledge lives in nodes, but
  a relation sometimes needs clarifying, and a link annotation has nowhere to put
  a body, evidence or incoming links.
- **A `RelationCard` is not a `ConceptCard`.** It contributes an edge and does
  not become a node. This reverses the earlier note's claim that a card about a
  relation "remains a concept node": were it both, every explained relationship
  would appear twice in the graph. It is still not a `Relation` either — see
  [relation is not a card](relation-not-a-card.md).
- The discriminator is an authored entry, so it is a claim the checker must
  verify against a contract. `cards | typed Relation | graph` was wrong once a
  card is never a `Relation`; the idiom is now `cards | typed RelationCard`.
- `KnowledgeGraph <: {Knowledge, Graph[Concept, Relation]}` is unchanged and is
  what the supertype set exists for — see [two domains](two-domains.md).

Stated in [knowledge model](../models/domain/knowledge.md),
[Card](../wiki/hql/types/card-type.hmd), [type system](../wiki/hql/type-system.hmd),
[Graph](../wiki/hql/types/graph-type.hmd) and [knowledge](../wiki/hql/knowledge.hmd).
See [field visibility](../wiki/hql/visibility.hmd) for what a card exposes.
