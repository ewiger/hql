# `metadata` is a card's knowledge layer

Recorded 2026-09-18 from a user instruction: metadata is used only with cards and
is truly a knowledge attribute — a knowledge-type element.

- `header : Data` is document-level and every document has one: name, path,
  format, title, the authored block, extension contributions about the file.
- `metadata : Data` is card-only and knowledge-level: the concepts, relations,
  propositions and evidence attached to the card. It is the addition that makes
  a card more than a document, so it belongs to the knowledge model.
- The two were tracked as competing spellings for one field (`INC-02`,
  `CON-04`). They are not: both items are now closed, because the names name
  two layers.
- Consequently the knowledge extension refines `card.metadata`, while `git`,
  database or indexer extensions contribute to `card.header`. Both are the same
  progressive-refinement mechanism over one unchanged tree.
- Superseded in one respect: the authored header entry does not "promote itself"
  into metadata. `doc.header.metadata` and `card.metadata` are two different
  fields. The first is the document's *optional* authored block; the second is
  assembled when a document becomes a card, from that block plus system metadata,
  extension contributions, `.hmd/**` vault state and other derived information.
  Both stay addressable and a query must know which it means — see
  [fields](../wiki/hql/fields.hmd).
- `metadata.knowledge.type` lives in this layer and is what distinguishes a
  `ConceptCard` from a `RelationCard` — see [card family](card-family.md).
- Open: what a card's metadata holds before an extension declares its shape, and
  what the system contributes to it at load beyond the authored block.

Recorded in [Card](../wiki/hql/types/card-type.hmd) and
[type system](../wiki/hql/type-system.hmd). See
[header, not frontmatter](header-not-frontmatter.md) and
[relation is not a card](relation-not-a-card.md).
