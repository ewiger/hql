<!-- hql reference: std/doc -->
# `std/doc`

```hql
module doc
```

Documents: a header of everything known about one and a body someone
wrote, and the card, a document a vault holds. Every declaration is a
type; the module declares no operation.

- [`Doc`](types/Doc.md) — A document: its `name`, `title`, `path` and `format`, its `header` and its `body`.
- [`HmdHeader`](types/HmdHeader.md) — The shape a hyper-markdown header declares over part of itself: an optional `metadata` tree.
- [`HmdDoc`](types/HmdDoc.md) — A document written in hyper-markdown: a `Doc` whose `header` is an `HmdHeader`, a partial schema in which other paths stay open.
- [`Card`](types/Card.md) — A card: knowledge given a physical document, the document a vault holds.
- [`HmdCard`](types/HmdCard.md) — A card whose body is written in hyper-markdown, `format` `Hmd`.
- [`MdCard`](types/MdCard.md) — A card whose body is written in Markdown, which no dialect type narrows.
<!-- hql reference: end -->
