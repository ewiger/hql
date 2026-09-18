# Semantic search is the read path of AI memory

Recorded 2026-09-18 from a user instruction: add a domain model for semantic
search, with the goal of building AI memory on top of `KnowledgeGraph`.

- It is written as a third model document,
  [semantic search](../models/domain/semantic-search.md), because its vocabulary
  — `Query`, `Embedding`, `Index`, `Hit`, `Ranking`, `Retrieval` — has no home in
  either peer domain. It is not a peer in the subtyping sense: it mints no node
  type and no edge type, so the rule in [two domains](two-domains.md) is not at
  risk. Nothing in it is anyone's parent.
- **A score is evidence, never relevance.** A hit carries the retrieval that
  produced it — query, index revision, model, metric, approximation — because the
  knowledge domain's single rule is that the rule used must be retained with the
  result. `Retrieval` is not optional on `Hit`.
- **An index contributes embeddings, never scores.** `card.metadata` is assembled
  and already names semantic-search state among its inputs, which is right for
  embeddings, chunk identity and staleness. A score exists only relative to a
  query, so it cannot live in a per-card layer. The sentence in
  [Card](../wiki/hql/types/card-type.hmd) that lists both as one kind of
  contribution is recorded as `INC-23`.
- **Cards are not concepts.** `semantic` ranks representations; `recall` ranks
  knowledge. A concept with no card has no text, so text search cannot reach it,
  and `Graph[Concept, Relation]` is parameterized by `Concept` for exactly that
  reason. `recall` is the AI-memory operation.
- **A `RelationCard` hit is not a seed.** It contributes an edge and is not a
  node — see [card family](card-family.md). Treating every hit as a node is the
  naive default and reintroduces the duplication the card split prevents.
- **Approximation is semantics, not optimization.** ANN search does not preserve
  observable values, so pushdown is illegal under the [pipes](../wiki/hql/types/pipes.hmd)
  optimizer rule unless `semantic` declares itself approximate in its own
  contract. Same for purity: without a pinned index revision, `semantic` is an
  effect.
- The pipeline is recommended as `cards | semantic(q) | take(5) | expand(depth = 1) | graph`,
  keeping `expand` as traversal and `graph` as the projection rather than
  settling the naming question in [graphs](../models/domain/graphs.md) by
  accident.
- The write path — remembering, reconciliation against existing concepts,
  consolidation of co-retrieval into derived relations, and eviction — is the
  undrawn effect boundary the knowledge model already names. Concept identity
  stops being theoretical there: a score may propose that two concepts are the
  same and may never decide it.

See [semantic search model](../models/domain/semantic-search.md) for the staged
implementation path.
