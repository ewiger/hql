<!-- hql reference: std/knowledge.RelationCard -->
# `RelationCard`

In [`std/knowledge`](../index.md).

```hql
type RelationCard : Card
```

A card whose subject is a relation: one whose `metadata` declares
`knowledge.type: Relation`, with a `knowledge.relation.source` and
`target` that each name a card of the vault. A graph projected from
cards takes its source and target as nodes, not the card itself.
<!-- hql reference: end -->
