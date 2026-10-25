# 0060 — A card keeps its path in the vault

Status: done
Related: [issue 0058](../0058-card-values-lose-their-provenance.md),
[`src/document.rs`](../../../src/document.rs),
[`src/sources.rs`](../../../src/sources.rs),
[`src/knowledge.rs`](../../../src/knowledge.rs)

The loader already gives every card system fields, set by it and kept
outside the authored header so that nothing a document writes can contradict
them: `name`, `path` and `format` on the document, `kind` and the
extensions' metadata (`lexical`) after it. `path` is the broken one. A source
supplies each document's path relative to the vault root,
`authors/frank-herbert.hmd`; `DocumentStore::from_source` joins it to the
root ([`src/document.rs`](../../../src/document.rs), `snapshot.root.join`) and
keeps only the result. So `path` is in whatever form the root was given, and
the same card reads differently by how the vault was found:

| Vault | `c.path` |
| --- | --- |
| `--vault examples/tutorial`, from the repository root | `examples/tutorial/authors/frank-herbert.hmd` |
| `--vault` an absolute path, from `/tmp` | `/Users/…/examples/tutorial/authors/frank-herbert.hmd` |
| discovered, from inside `examples/tutorial` | `/Users/…/examples/tutorial/authors/frank-herbert.hmd` |

`name` is vault-relative already: the file's stem, or its relative path where
two stems collide. A query result, a stored value or a diagnostic's `source`
that names a card by `path` depends on the working directory.

## Work

1. `path` holds the path the source supplied, relative to the vault root,
   the same however the vault was found. Where the file is on disk is the
   vault root joined with it, computed where a read or a render needs it.
2. A diagnostic's `source.path` and the loader's `skipped-document` origin
   name the document by the same vault-relative path.
3. A case pinning `c.path` from two working directories, written first as
   `proposed`.

Issue 0058 adds each header key's line to the same system fields.

## Done

`DocumentStore::from_source` keeps the path the source supplies, so `path` is
the card's place in the vault, as `lib/std/doc` declares it: `todo.hmd` with
`--vault examples/todo/wiki` and with `--vault examples/todo/wiki/../wiki`
alike, pinned by
[the card path session](../../../examples/todo/sessions/card-path.session).
`Vault::file_of` gives where a document is on disk. A diagnostic's `source`
and the loader's `skipped-document` origin name the same path.
