# 0040 — Configured data sources: `source("id")` and HQL-0012 accepted

Status: done
Related: [HQL-0012](../proposals/HQL-0012/README.md),
[HQL-0021](../proposals/HQL-0021/README.md),
[0037](0037-io-typed-decoding.md),
[the CLI model](../models/behavior/cli.md),
[the data sources examples](../../examples/sources/README.md)

HQL-0012 specifies sources a program names by id while the run context
holds every path, host and secret. Only the filesystem vault existed, and
`source` read endpoints alone.

## Scope

- `[sources.<id>]` in `hql.toml`, read with the vault in `src/sources.rs`
  and refused at load when malformed; `[execution] max_elements` and
  `max_bytes`.
- `source("id")` in `src/extensions/io.rs`: typed from the configuration at
  check time, read at evaluation, decoded as a declared element type.
- Local kinds: `vault`, `csv`, `json`, `json_lines`, `yaml`, `sqlite`.
- `Value::Vault` carries a configured vault, so two vaults are read side by
  side.
- `std/io/mod.hql` declares the `id` form of `source` and the remaining
  source contracts; binding accepts one declaration per alternative of a
  native choice input.

## Done when

- `tests/sources.rs` covers HQL-0012's test plan for the implemented kinds:
  id validation, a literal secret refused and `_env` accepted, an unknown
  key refused, two vault sources in name order, an unknown id with the
  nearest, a computed id and a URL refused, checking with the files gone,
  and both bounds.
- `examples/sources/` runs in the corpus.

## Left open

- A configured file or table is a `Seq<T>`, not a concrete source type such
  as `CsvFile`, and `Vault` does not narrow `VaultSource`.
- `tap("id")`, `member`, `get` over a keyed source, stream, live and network
  kinds, and native queries.
