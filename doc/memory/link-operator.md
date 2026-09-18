# Doc references and the link operator

Recorded 2026-09-18 from user instructions, refining the design conversation in
[sets and subtypes](../conversations/sets-n-subtypes.md).

- `[[..]]` on its own is a **doc reference** and nothing more. It references a
  document in the current namespace, in any format the vault holds — Markdown,
  HMD, JSON, YAML, HTML. No function wraps it; the earlier `resolve([[alice]])`
  spelling is gone from `doc/`.
- The **link operator** is `[[a]] -> [[b]] {..}`: `->` constructs an edge, the
  brackets name its endpoints, `{..}` is ordinary `Data` carried on it.
- Special case: `[[bob]] {..}` authored inside `alice` means
  `[[alice]] -> [[bob]] {..}` and adds that edge to the vault graph. If the edge
  already exists the braces update its annotation instead of adding a second
  edge, so an edge is identified by its endpoints and its data accumulates
  across every place the link is authored.
- The keys in `{..}` are the author's, never grammar. Edge, Graph, Link and
  Relation types belong to the graph and knowledge extensions, keeping the core
  small: core → graph → knowledge, with HMD beside them as a document source.
- This revises the claim in [Doc](../wiki/hql/types/doc-type.hmd) and
  [knowledge](../wiki/hql/knowledge.hmd) that a link is never an edge-property
  container. It may carry data; it may not acquire identity or assert meaning by
  carrying it. Significant relationships still become Relation Cards.
- Card-ness remains a checked narrowing, so a reference never yields a `Card`.
- Open: whether the annotation merge is per key or deep and what a conflicting
  key does (last-write-wins would make it depend on traversal order); how a
  failed resolution is carried (an endpoint may not resolve, and HMD treats that
  as a warning by design); whether `<-` exists; how `->` binds against `|`; and
  whether an HQL cell's edge is a value or an assertion into the graph.

Defined in [link-op](../wiki/hql/operators/link-op.hmd). See
[corpus direction](corpus-direction.md) for the binding forms.
