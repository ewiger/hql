# Bootstrap requirements

Status: implemented bootstrap; language design remains experimental.

- The language is **HQL — Hyper Query Language**. Repository and binary: `hql`;
  sources: `.hql`; ordinary code fences: `hql`. Hibernate's HQL naming collision
  is accepted. Specialized HyperMarkDown fences may eventually include
  `hql#query`, `hql#eval`, and `hql#declare`; these are not implemented.
- HQL is a typed functional language for querying **and computing** over
  structured knowledge, initially centered on HyperMarkDown.
- HyperMarkDown remains the authored/storage representation. HQL must have a
  pure language core separate from document parsing, resolution, and I/O.
- First milestone, **met**: parse, type-check, and evaluate literals and
  same-type numeric addition through the library and `hql eval`; check a file
  through `hql check`. `Int` and `Float` are separate types; neither widens.
- Second milestone, **met**: a program of statements and bindings; pipelines
  over a vault of documents; the card family; semantic retrieval and graph
  traversal; a reporting queue with configurable modes; and a command-line host
  with `run`, `repl`, `render`, standard input and JSON output. See
  [the grammar](../behavior/expressions.md), [the CLI](../behavior/cli.md) and
  [reporting](../behavior/reporting.md).
- Failures must be diagnostics, not panics or silent arithmetic wrapping.
- Rust edition 2024, cargo, clap derive, tests, and the grem knowledge structure
  are the bootstrap stack. Preserve the generated dormant control layer.

## Explicit non-goals

Still out of scope: module imports and `init.hql`, an HMD parser (documents are
read as Markdown with a fenced header, not through HyperMarkDown's own parser),
constraints and proofs, a standard library, diagrams, effects of any kind — so
no persistence and no `hql#declare` — editor integration, and a complete
language specification.

The original list also named graphs, a Card runtime, pipelines and functions.
Those are now implemented at the level [the grammar](../behavior/expressions.md)
records, which is a working subset and not the designed language.

See [architecture](../domain/architecture.md), [expression behavior](../behavior/expressions.md),
and [design direction](../../wiki/design-direction.hmd).
