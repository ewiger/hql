# HQL-0002: Semantic retrieval over a precomputed embedding index

**Status**: accepted
**Created**: 2026-09-18
**Source**: [semantic search](../../models/domain/semantic-search.md)

## Abstract

Retrieval splits into two extensions. `lexical` is the existing hashed
bag-of-words matcher, unchanged but honestly named. `semantic` scores a query
against embeddings computed ahead of time by a language model and stored in an
SQLite index, which a Python tool under `contrib/semantics/` produces and the
Rust binary only consumes. Both return `Ranking<Card>`, and every `Hit` carries
the `Retrieval` that produced it — query, index, model, model revision, metric,
and whether the search was approximate. Scoring is exact brute-force cosine over
stored vectors, so `approximate` is `false`. A vault names its index in
`hql.toml`; without one, `semantic` is an error naming the index and the command
that builds it, and never falls back to `lexical`.

## Motivation

`semantic` does not model semantics. It hashes words into 256 buckets with
FNV-1a, weights them by damped term frequency, normalises, and compares by
cosine. It matches shared spellings. Against `tests/fixtures/vault`:

```text
semantic("bearer token authorization")     → bearer-tokens 0.45, oauth 0.42
semantic("delegated credential handover")  → one hit at 0.07, on the word "scheme"
```

The second query is an ordinary English paraphrase of OAuth and finds nothing.

The name is the harm, not the algorithm. A `Hit` tells every consumer that a
model produced its score, and `hql.hashbag.v1` is not a model of meaning. A
score is evidence for the proposition that a card is relevant to a query — the
rule [knowledge](../../models/domain/knowledge.md) rests on — and evidence whose
provenance misdescribes it is worse than no evidence.

Two things follow, and they are separable. The lexical matcher is useful and
should keep existing under a name that describes it. `semantic` should mean what
it says, which needs a model, which does not belong in this binary.

This is step three of the path in
[semantic search](../../models/domain/semantic-search.md). It does not reach
step four.

## Goals

- A paraphrase query returns the right cards.
- What produced a score is recoverable from the score.
- `cargo test` stays offline, deterministic, and free of any Python dependency.

## Non-goals

- **No approximate search.** No ANN index, no top-k pushdown into a store.
  Step four of the path stays open, and `approximate` stays `false`. Pushing an
  approximate search into a source is currently forbidden by
  [pipes](../../wiki/hql/types/pipes.hmd), which requires an optimisation to
  preserve observable values; changing that needs its own proposal.
- **No chunking.** Card-level vectors only. A card is not a retrieval unit and a
  passage is, but the schema below only leaves room for it.
- **No `recall` over knowledge.** Concepts without cards stay unreachable, and
  the implementation says so rather than implying `semantic` covers them.
- **No write path.** No effects, no persistence from HQL.
- **No embedding computed in the Rust binary.** The producer is Python; the
  consumer is Rust.

## Specification

Depends on [HQL-0001](../HQL-0001/README.md): both are extensions under the
registry it defines, and both are `Pure`.

### 0. Where the corpus lives

`tests/fixtures/birds/` holds it. The suite must never depend on a tree that
may be absent, and a corpus the acceptance criterion rests on is part of the
suite rather than an illustration beside it.

- `lexical` provides `lexical(query : String)`. It is today's implementation,
  moved unchanged. Its `Retrieval.model` remains `hql.hashbag.v1` and its
  `revision` is the crate version. Nothing about it was wrong except its name.
  The name is `lexical` rather than `bm25` or `keywords` because it names the
  claim — scoring over shared spellings — and so stays true if the scoring
  inside it is ever upgraded.
- `semantic` provides `semantic(query : String)`. It MUST NOT fall back to
  `lexical` under any condition, because a silent fallback restores exactly the
  confusion this proposal removes.
- Both take `Set<Card>` or `Seq<Card>` and return `Ranking<Card>`.

### 2. Configuration

A vault names its index; the path MUST resolve inside the vault root.

```toml
[semantics]
index = ".hql/index.sqlite"
```

- With no `[semantics] index`, `semantic` MUST be a name error naming both the
  missing configuration and `hql-semantics build`.
- If `index_meta.vault` names a different vault than the one loaded, `semantic`
  MUST refuse rather than rank against a corpus it was not built for.

### 3. The index

Schema version `1`. Both implementations pin it, and the reader MUST refuse a
version it does not know.

```sql
CREATE TABLE model (
    id          TEXT    PRIMARY KEY,  -- "sentence-transformers/all-MiniLM-L6-v2"
    revision    TEXT    NOT NULL,     -- pinned model revision
    dimensions  INTEGER NOT NULL,     -- 384
    metric      TEXT    NOT NULL,     -- "cosine"
    normalized  INTEGER NOT NULL      -- 1 when vectors are unit length
);

CREATE TABLE index_meta (
    schema_version INTEGER NOT NULL,  -- 1
    vault          TEXT    NOT NULL,
    built_at       TEXT    NOT NULL,  -- ISO-8601 UTC
    tool_version   TEXT    NOT NULL
);

CREATE TABLE card (
    name         TEXT PRIMARY KEY,
    path         TEXT NOT NULL,       -- relative to the vault root
    content_hash TEXT NOT NULL,       -- sha256 of the embedded text, lowercase hex
    embedding    BLOB NOT NULL        -- f32 little-endian, `dimensions` of them
);
```

The schema MUST remain able to grow a `chunk` table without changing these
three. No `chunk` table is added here.

### 4. The embedded text

The text Python embeds and the text Rust hashes MUST be byte-identical. This is
the rule most likely to be broken and the one whose breakage has no symptom: a
mismatch produces an index that scores text nobody wrote, and every score is
quietly wrong.

```text
text(card) = title | authored header strings in key order | body   joined by " "
```

That is `Document::text()` in `src/document.rs`. The *authored* header only:
the derived `name`, `path` and `format` entries say where a document sits and
what it is called rather than what it says, and a card that moves between
directories must not thereby change its score. Both implementations MUST
implement it, and a change MUST change both in one commit. A golden test over
every card in the corpus MUST pin the agreement.

### 5. What a ranking reports

`Retrieval` gains `revision`. Every field MUST describe what actually ran.

- `model` — the model id from the `model` table, never a placeholder
- `revision` — the pinned model revision, and nothing else. It does not also
  carry the index's `built_at`: one field with two meanings is the confusion
  this proposal exists to remove. Which build answered is recoverable from
  `index`, and `built_at` stays in `index_meta` where the index owns it.
- `metric` — the metric computed
- `approximate` — `false`, because scoring is exact brute force

### 6. Staleness and coverage

Both conditions are reported through the queue in `src/reporting.rs`; neither is
silent, and neither is fatal.

- A card whose recomputed `content_hash` differs from the index has a vector for
  text that no longer exists. It MUST be ranked anyway and MUST queue a
  **warning** naming the card. A stale score is a wrong answer rather than an
  impossible one, which is what separates a warning from an error. Skipping it
  instead would make an edited card vanish from a ranking, which is a silence
  a reader cannot tell from irrelevance.
- A card absent from the index MUST be absent from the ranking, with one warning
  counting how many were skipped. It MUST NOT be scored zero, which is
  indistinguishable from indexed and unrelated.

### 6a. The reader

`rusqlite` with the `bundled` feature reads the index. Writing an SQLite reader
by hand is exactly the significant, well-understood work `doc/stack.md` says a
crate may remove, and `bundled` compiles a pinned SQLite rather than trusting
whichever one a machine happens to carry, which is what a committed fixture
needs. `#![forbid(unsafe_code)]` binds this crate and is unaffected.

### 7. The producer

`contrib/semantics/` is a standalone Python CLI. The Rust binary MUST NOT invoke
it, and `cargo test` MUST NOT require it.

```text
hql-semantics build   --vault DIR --out PATH [--model NAME] [--offline]
hql-semantics verify  --vault DIR --index PATH    # exit 1 when stale
hql-semantics inspect --index PATH
```

- Default model `sentence-transformers/all-MiniLM-L6-v2`, `384` dimensions.
- The model **revision** MUST be pinned, not only its name.
- Vectors MUST be L2-normalised, and `model.normalized` MUST record it.

## Backwards Compatibility

`semantic` changes meaning, which is the proposal's purpose. Every existing use
must either be rewritten to `lexical` or given an index. `tests/fixtures/vault`
has no index, so its queries move to `lexical`.

## Security Considerations

The index is a file the vault names and an SQLite database the binary parses, so
it is untrusted input: the reader MUST bound `dimensions`, MUST check that each
`embedding` blob is exactly `dimensions * 4` bytes, and MUST reject a schema
version it does not know rather than interpreting unknown columns. The path MUST
resolve inside the vault root. The Python side reaches the network to fetch a
model; `--offline` MUST make that impossible and MUST be what CI uses.

## Deployment / Activation

1. land [HQL-0001](../HQL-0001/README.md) first, registry and all
2. add the birds corpus, with `lexical` still in place
3. add `contrib/semantics/` and commit a generated index
4. rename the existing matcher to `lexical`
5. add the `semantic` extension reading the index
6. only then remove any remaining assumption that `semantic` needs no index

## Reference Implementation

- `tests/fixtures/birds/` — at least 40 documents: order, family and species
  cards; at least four relation cards, one with endpoints that deliberately do
  not resolve; `metadata.status` on a subset; `[[links]]` including one forward
  link to a document that does not exist
- `tests/fixtures/birds/.hql/index.sqlite` — generated, committed, about 61 KB
- `contrib/semantics/` — `cli.py`, `vault.py`, `embed.py`, `store.py`, `tests/`
- `src/extensions/lexical.rs` — today's `src/search.rs`, renamed
- `src/extensions/semantic.rs` — the SQLite reader and the scorer
- `src/search.rs` — `Retrieval` gains `revision`

## Test Plan

The articles MUST be written so the difference is measurable. Describe owls as
"nocturnal", "after dark" and "low light", and never as hunting "at night"; then
`lexical("birds that hunt at night")` finds nothing useful and `semantic` finds
the owls. At least **three** such pairs, asserted by card name. If these are
weak, the rest of the proposal does not compensate.

Unit tests MUST include:

- the golden text test: Python and Rust agree byte for byte on every card
- a truncated `embedding` blob rejected, not read past
- an unknown `schema_version` refused with the exact reason `unknown index schema`
- a stale `content_hash` producing a warning and still ranking
- a card absent from the index absent from the ranking, with the count warned

Integration tests MUST include:

- the three paraphrase pairs
- `semantic` with no configured index failing with a name error naming
  `hql-semantics build`
- `cargo test` passing on a machine with no Python

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cd contrib/semantics && pytest
```

## Changelog

- 2026-09-18: drafted
- 2026-09-18: accepted; the five open questions settled into the specification —
  the corpus lives in `tests/fixtures/birds/`, a stale vector warns and still
  ranks, `rusqlite` with `bundled` is the reader, `lexical` keeps its name, and
  `revision` carries the model revision alone
- 2026-09-18: the embedded text is the *authored* header, because
  `Document::text()` was including the derived `path` and so scoring a card on
  its filename
