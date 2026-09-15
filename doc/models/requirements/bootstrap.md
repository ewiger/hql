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
- First milestone: parse, type-check, and evaluate literals and integer addition
  through the library and `hql eval`; check a file through `hql check`.
- Failures must be diagnostics, not panics or silent arithmetic wrapping.
- Rust edition 2024, cargo, clap derive, tests, and the grem knowledge structure
  are the bootstrap stack. Preserve the generated dormant control layer.

## Explicit bootstrap non-goals

No graphs, Card runtime, HMD parser or renderer, resolver, module imports,
functions, pipelines, constraints/proofs, transformations, standard library,
diagrams, computational cells, editor integration, or complete language spec.

See [architecture](../domain/architecture.md), [expression behavior](../behavior/expressions.md),
and [design direction](../../wiki/design-direction.hmd).
