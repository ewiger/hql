<!-- hql reference: std/knowledge -->
# `std/knowledge`

```hql
module knowledge
```

Knowledge: the semantic domain, where everything is a claim. Every
declaration is a type; the module declares no operation.

- [`Concept`](types/Concept.md) — Something identified or described: the type that puts a card in a node position.
- [`Proposition`](types/Proposition.md) — A claim that can be supported, disputed or constrained.
- [`Evidence`](types/Evidence.md) — Material offered for or against a claim.
- [`Knowledge`](types/Knowledge.md) — The universe of concepts, relations, propositions and evidence.
- [`Relation`](types/Relation.md) — A meaningful relation from `source` to `target`: an `Edge` narrowed with identity, evidence and provenance.
- [`ConceptCard`](types/ConceptCard.md) — A card whose subject is a concept: a `Card` that is a `Concept`, and so a node.
- [`RelationCard`](types/RelationCard.md) — A card whose subject is a relation: one whose `metadata` declares `knowledge.type: Relation`, with a `knowledge.relation.source` and `target` that each name a card of the vault.
- [`KnowledgeGraph`](types/KnowledgeGraph.md) — A graph whose nodes are concepts and whose edges are relations, which is still `Knowledge`: the projection loses neither contract.
<!-- hql reference: end -->
