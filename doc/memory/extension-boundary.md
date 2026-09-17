# Extension boundary and card-ness

Recorded 2026-09-17 from a user instruction that tightens
[HMD integration](../wiki/hmd-integration.hmd).

- HQL is a general language. It names no source in its core. Databases, graph
  stores, Postgres, bulk storage such as HDFS and filesystem documents are peers,
  each reached through an extension. HMD is one extension, not the subject.
- Layering is core → graph → knowledge. The knowledge extension depends on the
  core graph capabilities; nothing depends on HMD.
- HMD is a format and a set of documents on a filesystem. Plain Markdown is also
  a collection of documents, and both project to `Doc`. There is no divide under
  which `.hmd` files are cards and `.md` files are merely documents.
- `Card` is an additional data type that gives a piece of knowledge a physical
  representation. A card can reference the concepts it explains. Card-ness is a
  knowledge notion, not a file format.
- Consequently the format condition was removed from the `Card` narrowing in
  [Card](../wiki/hql/types/card-type.hmd); the `Hmd` prefix in that type name now
  outlives its reason and the rename is unsettled. Whether a card still requires
  a vault is a separate open question — naming and `downlinks` do need a
  namespace.
