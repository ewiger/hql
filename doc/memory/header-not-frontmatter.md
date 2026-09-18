# The field is `header`, never `frontmatter`

Recorded 2026-09-18 from a user instruction to replace `frontmatter` with
`header` across `doc/`.

- `frontmatter` is HyperMarkDown's and Markdown's jargon for one format's way of
  writing the authored layer: a YAML block fenced by `---`. It is not the name of
  the field, and there is no `Frontmatter` type in HQL.
- A document's header is one `Data` tree of derived, authored and contributed
  entries. A format that supplies metadata some other way, or not at all, still
  has a header — which is the whole argument for the name.
- The word survives in `doc/` only where it names that block as another system's
  term: the definitional passage in [Doc](../wiki/hql/types/doc-type.hmd), a link
  to HMD's `frontmatter.py`, and quotations in
  [inconsistencies](../status/inconsistencies.md), which is an audit record of
  former states and must not be rewritten.
- The vocabulary tables in [core](../wiki/hql/core.hmd) and
  [knowledge](../wiki/hql/knowledge.hmd) listed `Frontmatter` as a document type;
  they now list `Doc`, since the header is `Data` and not a type of its own.
- `metadata` is not a competing spelling. It is a card-only, knowledge-level
  layer beside the document-level `header`, so `INC-02` is resolved rather than
  narrowed, and `CON-04` is closed. See [knowledge metadata](knowledge-metadata.md).
