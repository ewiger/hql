<!-- hql reference: std/doc.Card -->
# `Card`

In [`std/doc`](../index.md).

```hql
abstract type Card : Doc { metadata : Data // the knowledge attached to this card, assembled kind : String // "ConceptCard" or "RelationCard" }
```

A card: knowledge given a physical document, the document a vault
holds. Its `metadata` is the knowledge attached to it, assembled, and
its `kind` is `"ConceptCard"` or `"RelationCard"`, the two kinds module
`knowledge` declares, which say what the document is about.
<!-- hql reference: end -->
