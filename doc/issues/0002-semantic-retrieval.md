# 0002 — Semantic retrieval over a real embedding index

Status: done
Decision: [HQL-0002](../proposals/HQL-0002/README.md)
Branch: `feat/semantics`, off `feat/extensions`
Blocked by: [0001](0001-extension-mechanism.md)

The work that carries out [HQL-0002](../proposals/HQL-0002/README.md). The
proposal holds the design — the SQLite schema, the text contract, what a ranking
must report, and what is out of scope. This card holds the sequence and what
"done" means.

Do not start before [0001](0001-extension-mechanism.md) has landed. Both
`lexical` and `semantic` are extensions under the registry it adds, and building
them as two more entries in the core's `match` is the thing the split exists to
prevent.

## Sequence

Four commits, in this order.

1. **`tests/fixtures/birds/`** — the corpus. At least 40 documents, described in
   the proposal under Reference Implementation. Real encyclopedic prose, three to
   eight sentences each; the articles are what a model will embed, so templated
   text leaves nothing to measure. Leave `tests/fixtures/vault` alone — the
   existing suite pins behaviour against it.
2. **`contrib/semantics/`** — the Python CLI, and the generated index committed
   at `tests/fixtures/birds/.hql/index.sqlite`.
3. **`lexical`** — today's matcher, moved to an extension and renamed. Existing
   uses of `semantic` against `tests/fixtures/vault` move with it.
4. **`semantic`** — the SQLite reader and the scorer, plus `revision` on
   `Retrieval`.

## The acceptance criterion

Write the corpus so the difference is measurable, as the proposal's Test Plan
requires: owls described as "nocturnal" and "after dark", never as hunting "at
night". Then `lexical("night hunting birds")` finds nothing useful and
`semantic` finds the owls. At least three such pairs, asserted by card name.

This is the whole point of the issue. If these tests are weak, nothing else in
it compensates.

## Done when

- The three paraphrase pairs pass.
- `cargo test --locked` passes on a machine with no Python installed.
- `hql-semantics build --built-at …` regenerates the committed index
  reproducibly and `hql-semantics verify` reports it clean.
- `cargo clippy --locked --all-targets -- -D warnings` and `cargo fmt --check`
  are clean.
- Documentation: `doc/models/domain/semantic-search.md` moves step three to done
  and records why step four is untouched; `doc/wiki/hql/extensions/semantic.hmd`
  and `lexical.hmd` exist and are real, not the zero-byte stubs currently in that
  folder; `doc/stack.md` records the SQLite crate and a `contrib/` section;
  `doc/status/inconsistencies.md` closes `INC-23`, which said an indexer
  contributes embeddings but not scores — this is that split implemented; and
  `README.md` uses the birds vault as the worked example. State each decision in
  the card or model that owns it; `doc/memory/` is for what stays undigested —
  see its [README](../memory/README.md).

## Settle before starting

The five Open Questions at the end of [HQL-0002](../proposals/HQL-0002/README.md).
