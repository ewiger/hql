# Graphs and knowledge are two peer domains

Recorded 2026-09-18 from a user instruction, correcting the supertype-set form
`Relation[S, T] <: {Card, Edge[S, T]}` and asking for a document per domain.

- `Relation[S, T] <: {Card, Edge[S, T]}` failed because a Relation Card
  *represents* a relation and is not one — representation is not ancestry. The
  reason was not that the parents came from two domains. `Relation` and `Edge`
  are each true supertypes within their own domain, knowledge and graphs, which
  is not the same as one type having both as parents. See
  [relation is not a card](relation-not-a-card.md).
- A supertype set spanning the two domains is legitimate where the type really
  is both, and that is the goal:
  `KnowledgeGraph <: {Knowledge, Graph[Concept, Relation]}`. A knowledge graph
  is a graph of concepts and relations and is still knowledge, retaining
  conflicting claims, provenance and evidence. It makes the graph projection
  lossless, and it is the case that forces `{A, B}` to be a real intersection
  rather than a list of declared parents — an open question in
  [type system](../wiki/hql/type-system.hmd).
- The knowledge side of this has since been refined: `Card` is a base
  representation type, and it is `ConceptCard <: {Card, Concept}` that occupies a
  node position while `RelationCard <: Card` supplies an edge. See
  [card family](card-family.md). `KnowledgeGraph` is unchanged.
- The two domains are peers, not layers: neither contains the other, and a type
  in one is never a supertype of a type in the other. The graph domain is
  structural; the knowledge domain is semantic and everything in it is a claim.
- Each now has a dedicated model document: [graphs](../models/domain/graphs.md),
  written first, and [knowledge](../models/domain/knowledge.md), rewritten from
  the material already scattered across the wiki cards, the memory notes and the
  [design conversation](../conversations/sets-n-subtypes.md).
- The wiki cards stay the type-level record; the model documents are the
  domain-level one. Cross-links run both ways.
