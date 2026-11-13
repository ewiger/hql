# 0061 — A configured id names an endpoint: `[endpoint.<id>]`

Status: done
Branch: `feat/kafka-postgres`
Related: [HQL-0046](../../proposals/HQL-0046/index.hmd),
[HQL-0035](../../proposals/HQL-0035/index.hmd),
[HQL-0012](../../proposals/HQL-0012/index.hmd)

An id declared in `hql.toml` is read by `source` and `tap` and written by
`target` and `sink`. The direction belongs to the function, not the
declaration, so the declaration says nothing about it. What it declares is
an **endpoint**, the term the language already uses for where bytes are, with
`Readable` and `Writable` as its capabilities.

## Work

1. `hql.toml` declares one under `[endpoint.<id>]`, with
   `[endpoint.<id>.fields.<field>]` for its boundary. `[sources]` is refused,
   naming `[endpoint.<id>]`.
2. Diagnostics name a configured id as an endpoint: ``no endpoint `count` is
   configured``, ``endpoint `ledger`: ...``, `"..." is not an endpoint id`.
3. In Rust, the configuration lives in `src/endpoints.rs` as `Endpoints`,
   held by `Vault::endpoints`; `src/sources.rs` keeps only how a vault's
   documents are read.
4. `examples/sources/` becomes `examples/endpoints/`, `tests/sources.rs`
   becomes `tests/endpoints.rs`.
5. The user pages, the models and the proposals that state the
   configuration say `[endpoint.<id>]` and "configured endpoint"; the demos
   in `hql-demo` follow.

"Source" keeps its meaning where it means what `source` reads:
`DataSource`, `FiniteSource`, `LiveSource`, and a vault's documents.

## Done when

- No tracked file outside `doc/issues/archive/` and the released sections
  of `CHANGELOG.md` writes `[sources.`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
