# 0001 — Real embeddings, and renaming the lexical matcher

Status: todo
Created: 2026-09-18
Branch: `feat/semantics` (new, off `feat/lang-design`)

This file is the implementation brief. It is written to be read cold, in a new
session, by someone who has not seen the conversation it came from.

## Why

`semantic(query)` in `src/search.rs` does not model semantics. It is a hashed
bag of words — FNV-1a into 256 buckets, log-damped term frequency, L2-normalised,
compared by cosine. It matches **shared spellings**, not meaning. The
demonstration that settles it, against `tests/fixtures/vault`:

```text
semantic("bearer token authorization")        → bearer-tokens 0.45, oauth 0.42
semantic("delegated credential handover")     → one hit at 0.07, on the word "scheme"
```

The second query is an ordinary English paraphrase of OAuth and finds nothing.
A step named `semantic` that cannot survive a synonym is misnamed, and the name
is the part that misleads: every consumer of a `Hit` is told a model produced
the score, and `hql.hashbag.v1` is not a model of meaning.

Two things follow, and they are separable:

1. the lexical matcher is useful and should keep existing, under an honest name;
2. `semantic` should mean what it says, which needs real embeddings, which needs
   a model, which does not belong in this Rust binary.

This is step three of the path in
[semantic search](../models/domain/semantic-search.md) — the index extension.
It does **not** reach step four: scoring stays exact brute-force cosine over
stored vectors, so `approximate` stays `false` and the pushdown question stays
open.

## Scope

Three deliverables, in dependency order. Each is independently testable and
should land as its own commit.

### (1) A birds vault — `tests/fixtures/birds/`

A worked example large enough to be worth querying: **at least 40 documents**,
about birds, with a real taxonomy.

The current `tests/fixtures/vault` has six cards and cannot demonstrate
retrieval, traversal or ranking at any interesting scale. Do not replace it —
the existing suite pins behaviour against it. Add the birds vault beside it.

Structure it so it exercises the whole card family:

- **Order cards** — Passeriformes, Accipitriformes, Anseriformes, Strigiformes,
  Apodiformes, Piciformes, Charadriiformes, and others as needed.
- **Family cards** — Corvidae, Paridae, Turdidae, Accipitridae, Anatidae,
  Strigidae, Trochilidae, Picidae, Laridae.
- **Species cards** — the bulk of the 40: common raven, Eurasian blue tit,
  common blackbird, golden eagle, mallard, barn owl, tawny owl,
  ruby-throated hummingbird, great spotted woodpecker, herring gull, and so on.
- **Relation cards** — `metadata.knowledge.type: Relation` with
  `knowledge.relation.source` and `.target`, for relationships that need
  explaining rather than a bare link: a disputed placement, a case of convergent
  evolution, a recent split or lump. At least four, and at least one whose
  endpoints deliberately do **not** resolve, so the fallback-to-`ConceptCard`
  path stays covered.
- **`metadata.status`** on a useful subset (`todo`, `done`, `stub`), so the
  to-do-list query in `tests/fixtures/board.hmd` has something to say.
- **Links** in bodies, `[[target]]`, including at least one forward link to a
  document that does not exist, so the warning path stays covered.

Write real prose. Three to eight sentences per article, in a consistent
encyclopedic register. The articles are the corpus a language model will embed,
so thin or templated text leaves nothing to measure.

**The articles MUST be written so that lexical search demonstrably fails and
embedding search demonstrably succeeds.** This is the acceptance criterion for
the whole issue, so design for it: write the owl articles using
"nocturnal", "after dark", "low light", and never the phrase "hunt at night".
Then `lexical("birds that hunt at night")` finds nothing useful and
`semantic("birds that hunt at night")` surfaces the owls. Produce **at least
three** such query pairs and pin them as tests.

### (2) `contrib/semantics/` — the Python tool

A standalone Python CLI that reads a vault and writes an SQLite index. It is a
build-time producer; the Rust binary is a consumer and never invokes it. `cargo
test` MUST NOT depend on Python being installed.

```text
contrib/semantics/
  README.md
  pyproject.toml
  src/hql_semantics/__init__.py
  src/hql_semantics/cli.py       # argument parsing only
  src/hql_semantics/vault.py     # document reading, mirroring src/document.rs
  src/hql_semantics/embed.py     # the model
  src/hql_semantics/store.py     # the SQLite writer
  tests/
```

Commands:

```text
hql-semantics build   --vault DIR --out PATH [--model NAME] [--offline]
hql-semantics verify  --vault DIR --index PATH      # staleness report, exit 1 if stale
hql-semantics inspect --index PATH                  # model, revision, dimensions, count
```

Default model: a small sentence-transformer such as
`sentence-transformers/all-MiniLM-L6-v2` (384 dimensions). Pin the **revision**,
not just the name. Normalise vectors to unit length and record that it was done.

**The embedded text MUST be byte-identical to what Rust hashes.** Pin the
contract in both implementations and test it with a golden fixture: today
`Document::text()` in `src/document.rs` is the title, then every string in the
header, then the body, joined by single spaces. If that definition changes,
it changes in both places in the same commit. A mismatch here produces an index
that scores the wrong text with no symptom, so it fails silently and stays
wrong.

### (3) Rename, and consume the index

- **`lexical(query)`** is the current hashed-bag matcher, moved and renamed
  unchanged. It keeps returning `Ranking[Card]`, and its `Retrieval.model` stays
  `hql.hashbag.v1`. Nothing about it was wrong except its name.
- **`semantic(query)`** requires an embedding index. With none configured it is
  an error naming the missing index and the command that builds one — never a
  silent fallback to `lexical`, which would restore the confusion this issue
  removes.

Configure the index per vault, in the `hql.toml` the reporting layer already
reads:

```toml
[semantics]
index = ".hql/index.sqlite"
```

## SQLite schema

Pin it exactly. Both sides implement this; `schema_version` gates compatibility
and the Rust reader refuses a version it does not know.

```sql
CREATE TABLE model (
    id          TEXT    PRIMARY KEY,  -- "sentence-transformers/all-MiniLM-L6-v2"
    revision    TEXT    NOT NULL,     -- pinned model revision
    dimensions  INTEGER NOT NULL,
    metric      TEXT    NOT NULL,     -- "cosine"
    normalized  INTEGER NOT NULL      -- 1 when vectors are unit length
);

CREATE TABLE index_meta (
    schema_version INTEGER NOT NULL,  -- 1
    vault          TEXT    NOT NULL,  -- the vault root it was built from
    built_at       TEXT    NOT NULL,  -- ISO-8601 UTC
    tool_version   TEXT    NOT NULL
);

CREATE TABLE card (
    name         TEXT PRIMARY KEY,    -- the card's name in its namespace
    path         TEXT NOT NULL,       -- relative to the vault root
    content_hash TEXT NOT NULL,       -- sha256 of the embedded text, lowercase hex
    embedding    BLOB NOT NULL        -- f32 little-endian, `dimensions` of them
);
```

Card-level vectors only. Chunking is a real open question in
[semantic search](../models/domain/semantic-search.md) — a card is not a
retrieval unit, a passage is — and it is explicitly **out of scope here**. Leave
the schema able to grow a `chunk` table later; do not add one now.

## What the index must report

None of these is optional; they are what makes a score usable as evidence.

- **`Retrieval` reports what actually ran.** `model` is the real model id,
  `revision` is the pinned revision, `metric` is what was computed,
  `approximate` is `false` because scoring is exact brute force over stored
  vectors. Add `revision` to the `Retrieval` type.
- **Staleness is detected and said out loud.** Rust recomputes the content hash
  and compares. A card whose hash differs from the index has a vector for text
  that no longer exists. Recommended: rank it anyway and queue a **warning**
  naming the card, because a stale score is a wrong answer rather than an
  impossible one, and the reports queue in `src/reporting.rs` already handles
  that distinction. See Open Questions.
- **A card absent from the index is absent from the ranking**, with a warning
  saying how many were skipped. Never score it as zero: that is indistinguishable
  from "indexed and unrelated".
- **Horizon.** `cards | semantic(q)` ranks the cards it was given, intersected
  with the index. If the index covers a different vault than the one loaded,
  refuse rather than guess.

## Tests

`cargo test` stays hermetic, offline and deterministic — `doc/stack.md` requires
it, and that requirement is why the fake was built in the first place.

- **Commit a prebuilt index** at `tests/fixtures/birds/.hql/index.sqlite`,
  generated by the Python tool. At 40 cards × 384 dimensions × 4 bytes it is
  about 61 KB, which is a reasonable thing to keep in the repository.
- **Rust tests** read that file. They never run Python.
- **Python tests** (`pytest`) cover parsing, hashing and the writer, and run
  separately. Add a CI note; do not wire them into `cargo test`.
- **The paraphrase tests** are the acceptance criteria. At least three cases
  where `lexical` returns nothing useful and `semantic` returns the right cards,
  asserted by name.
- **A golden text test** proving the Python and Rust `text()` implementations
  agree, byte for byte, on every card in the birds vault.
- **A staleness test**: edit a card in a temporary copy, and assert the warning.

## Documentation to update

Not optional, and not a separate pass. `doc/` is the knowledge base this project
is built around, and leaving it stale is how `INC-15` and `CON-05` happened.

- `doc/models/domain/semantic-search.md` — step three moves to done; record the
  `lexical` / `semantic` split; note that step four is still untouched and why.
- `doc/models/behavior/cli.md` — the new step, and the `[semantics]` key.
- `doc/wiki/hql/extensions/semantics.hmd` — currently absent; the extension card.
  (`doc/wiki/hql/extensions/` holds three zero-byte stubs; this one should be
  real.)
- `doc/stack.md` — the new Rust dependency, and a section for `contrib/`
  covering the Python side. Note that `doc/stack.md` governs the implementation
  and not HQL's design; the Python tool is implementation.
- `doc/memory/` — a note recording what this settled, in the style of
  `doc/memory/cli-host.md`. In particular: that a fake search was renamed rather
  than deleted, and why.
- `doc/status/inconsistencies.md` — `INC-23` said an indexer contributes
  embeddings but not scores. This issue implements exactly that split; close it.
- `README.md` — the birds vault as the worked example.

## Non-goals

State them in the memory note too, so the next reader does not assume otherwise.

- No approximate search, no ANN index, no top-k pushdown into a store. Step four
  of the path stays open, and `approximate` stays `false`.
- No chunking; card-level vectors only.
- No `recall` over knowledge — cardless concepts stay unreachable.
- No write path, no effects, no persistence from HQL.
- No embedding computed inside the Rust binary. The producer is Python; the
  consumer is Rust. That is the extension boundary
  `doc/memory/extension-boundary.md` draws, and it is why this works.

## Open questions — settle these with the user before building

1. **Where the birds vault lives.** Recommended `tests/fixtures/birds/`, because
   `examples/` is not present in every working tree and `tests/corpus.rs`
   already has to skip when it is missing. The alternative is
   `examples/vaults/birds/`, which is where it semantically belongs.
2. **What a stale vector does** — warn and rank (recommended), warn and skip, or
   error. This is a reporting-severity question and the machinery already exists.
3. **The Rust SQLite dependency.** `rusqlite` with the `bundled` feature is the
   obvious choice and compiles SQLite into the binary. `doc/stack.md` says to
   prefer the standard library and add a crate only when it removes significant,
   well-understood work; this qualifies, but it is a stack change and needs
   recording.
4. **Whether `lexical` is the right name.** Alternatives: `matching`, `keywords`,
   `bm25` if the scoring is upgraded. The requirement is only that the name does
   not claim meaning.
5. **Whether `semantic` without an index is a name error or a type error.**
   Recommended: name error, matching how `cards` reports a missing vault.

## Done when

- `hql --vault tests/fixtures/birds run <a paraphrase query>` returns the right
  birds, and the same query through `lexical` does not.
- `cargo test` passes with no Python installed.
- `hql-semantics build` regenerates the committed index reproducibly, and
  `hql-semantics verify` reports it clean.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean.
- Every file in **Documentation to update** has been updated.
