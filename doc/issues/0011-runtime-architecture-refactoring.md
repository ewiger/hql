# 0011 — Refactor runtime architecture before expanding HQL

Status: done

Carry out the five improvements in the supplied semantic-search branch review,
preserving HQL behavior except where the review identifies incorrect behavior.
Keep retrieval carriers and the Ranking fields described in `doc/memory/`.

## Steps

- [x] Introduce checked execution IR; resolve steps and constructors once,
  retain expression types and spans, and evaluate typed arguments and lambdas.
  Preserve checked collection and map result types (also resolves 0007).
- [x] Separate filesystem source loading, document naming/parsing, card
  assembly, knowledge edges, and extension metadata contributions.
- [x] Replace the YAML subset parser with a maintained YAML parser and an
  explicit fallible YAML-to-Data conversion; report unsupported data at loading.
- [x] Move immutable shared runtime values to Arc and assert Send + Sync at
  runtime and extension boundaries.
- [x] Replace handwritten step signature strings with structural contracts
  used for checking and help; retain hooks for dependent operations.

## Validation

Add regressions for checked/runtime type agreement, empty maps, lexical
resolution and lambda scopes, source-independent assembly, YAML semantics and
loader diagnostics, shared runtime values, and extension argument contracts.
Run `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`.

Final validation: 165 tests pass across 17 suites; `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and `git diff --check` pass.

## Progress

- Typed execution owns resolved steps and fields, constructor result types,
  binding views, typed lambda bodies, and source spans. Evaluation no longer
  resolves imports or repeats collection type joins. This completes 0007.
- Source snapshots feed document naming/parsing and knowledge assembly;
  registered extension hooks contribute their own metadata. Memory and filesystem
  sources use the same assembly path.
- YAML now uses `serde_yaml_ng` with explicit conversion to Data. Invalid
  documents are skipped with path-qualified diagnostics; flow annotations use
  the same parser, including nested structures and quoted commas.
- Shared runtime storage uses Arc. Tests assert Send + Sync and execute
  concurrent queries against a shared vault with shared retrieval provenance.
- All 18 registered steps declare structural contracts used by checking and
  generated help. Ordinary steps have no custom checker. Hooks cover ordering,
  narrowing, vault availability, and semantic-index configuration.
- Contract tests exposed and fixed result-shape mismatches in `typed` on lists
  and `filter` on rankings; filtered rankings retain their provenance and fields.
- Baseline repairs: corrected `Cocept` in `std/knowledge.hql` and supplied its
  document declarations in the individual-file standard-library test.

## Compatibility and boundaries

`Document::parse` and `data::parse_header` now return Result; Rust callers must
handle loader errors. Public runtime carriers use Arc instead of Rc, and
catalogue signatures are generated Strings with a structural spec alongside.
Excess, duplicate, or unknown step arguments now fail checking rather than being
ignored. No scheduler, SQL planner, first-class functions, or narrower retrieval
carriers were added. Ranking remains available with its existing fields.
