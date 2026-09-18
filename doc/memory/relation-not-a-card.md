# `Relation` is not a `Card`

Recorded 2026-09-18 from a user instruction: `Relation[S, T] <: Card` is
explicitly never true and had to be dropped everywhere, with `Relation` kept as a
first-class high-level type.

- `Relation` belongs to the knowledge vocabulary beside `Concept`, `Proposition`
  and `Evidence`. It has typed endpoints, its own identity, evidence and
  provenance, and it acquires none of that from a document: a relation may be
  imported, derived by a rule or read from a store with no card anywhere.
- A **Relation Card** is a card that *represents* a relation, reached as
  `card.metadata.relation : Relation[S, T]`. That is progressive refinement of
  the card's knowledge layer, not narrowing — see
  [knowledge metadata](knowledge-metadata.md).
- What a relation does narrow is the edge: `Relation[S, T] <: Edge[S, T]` adds
  identity and evidence to source, target and data. A knowledge graph is a
  multigraph because that identity is the relation's own, not because a card
  supplies it.
- The supertype-set form `Relation[S, T] <: {Card, Edge[S, T]}` is withdrawn with
  the rest. The `{A, B}` syntax itself stands; this case simply never motivated
  it.
- Preferring a Relation Card over an anonymous annotated edge for a significant
  relationship is unchanged — see [link operator](link-operator.md).

Edited out of [Card](../wiki/hql/types/card-type.hmd),
[Graph](../wiki/hql/types/graph-type.hmd),
[knowledge](../wiki/hql/knowledge.hmd) and the
[knowledge model](../models/domain/knowledge.md). Stated in
[type system](../wiki/hql/type-system.hmd).
