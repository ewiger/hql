# HQL-0002: Semantic retrieval over a precomputed embedding index

**Status**: accepted
**Created**: 2026-09-18
**Source**: [semantic-search requirements](../../models/requirements/semantic-search.md)

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

Which model computes those embeddings is the vault's choice. An embedder is
either `local` — a checkpoint loaded into the producer's own process — or `http`,
an OpenAI-shaped `/v1/embeddings` endpoint, which covers a hosted provider and a
model server on `localhost` equally. A vault declares one in `hql.toml`, and the
choice reaches the producer only: the consumer still has no model, no network and
no key, and reads what produced a score out of the index rather than out of
configuration.

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
[semantic search](../../models/requirements/semantic-search.md). It does not reach
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
  consumer is Rust. This is what makes the `query` table necessary, and it is
  therefore what limits `semantic` to the questions an index was built for.
  Lifting that limit means giving the consumer an embedder — an HTTP client, a
  key, a network failure mode and a score whose provenance no longer pins a
  revision — and it stays deferred to a proposal of its own. Choosing the
  embedder here changes *which* model fills an index, never *when* a vector is
  computed.
- **No embedder in the consumer's configuration.** `[semantics.embedder]` is
  read by the producer alone. The binary MUST ignore it, because a `Retrieval`
  that quoted configuration would be describing what a vault claims rather than
  what ran, and the index already records what ran.
- **No provider-specific scoring.** Every embedder normalises to unit length and
  is compared by cosine. A provider that cannot is out of scope rather than a
  second metric in the reader.

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

A vault MAY also declare which embedder fills that index. The whole table is
optional, and its absence means the `local` default.

```toml
[semantics.embedder]
provider    = "http"                                  # local | http
model       = "text-embedding-3-small"
revision    = "2024-01-25"                            # required for `http`
endpoint    = "https://api.openai.com/v1/embeddings"  # required for `http`
api_key_env = "OPENAI_API_KEY"                        # the variable's NAME
dimensions  = 1536                                    # optional declared width
```

- Both files stay as [issue 0011](../../issues/0011-wire-the-semantic-search-toolchain.md)
  divides them: `.hmd/config.toml` is the namespace, `hql.toml` is the index —
  and an embedder is how an index comes to exist, so it belongs here rather than
  in a third file or in flags a contributor has to remember.
- The producer reads this table; command-line flags override it, field by field.
  Nothing else reads it, and the binary MUST tolerate it without understanding
  it.
- An unknown `provider`, or an unknown key in the table, MUST be refused by
  name. A typo that silently selects a default is how a vault comes to be
  indexed by a model nobody chose.
- `api_key_env` names an environment variable. A key MUST NOT be writable into
  this file: a literal `api_key` key MUST be refused and MUST name `api_key_env`
  in the refusal. A vault's `hql.toml` is a committed file, and
  `tests/fixtures/birds/hql.toml` already is.
- `dimensions`, where declared, is an expectation the producer checks against
  what arrives. A provider that quietly changes a model's width is otherwise a
  rebuilt index that scores against a different geometry.

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

CREATE TABLE query (
    text      TEXT PRIMARY KEY,
    embedding BLOB NOT NULL        -- the same f32 little-endian layout
);
```

The `query` table is what makes the rest of this proposal implementable. A
consumer with no model cannot embed a document *or* a question, and inventing a
vector for one would be exactly the silent wrongness the split exists to avoid.
So the producer embeds the queries too — `hql-semantics build --query "…"`,
repeatable — and `semantic` looks one up. A query the index has no vector for
MUST be a failure naming that command, and MUST NOT be answered with a ranking
of nothing.

The consequence is a real limit, and it is the price of keeping the model out of
the binary: this implementation answers the questions an index was built for.
Embedding a query as it is asked needs a model at the consumer, which is a
proposal of its own.

The schema MUST remain able to grow a `chunk` table without changing these
four. No `chunk` table is added here.

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

### 5a. What the scorer requires of a vector

Scoring is a dot product. That is cosine similarity *only* for unit-length
vectors, so the precondition MUST be checked rather than assumed — with several
embedders writing indexes, the producer that normalises and the reader that
relies on it are no longer the same decision.

- The producer MUST L2-normalise every vector it stores, whatever the embedder
  returned and whatever the embedder claims. Truncated embeddings are the case
  that makes this concrete: asking a provider for fewer dimensions than its model
  computes returns a prefix of a unit vector, which is not itself unit-length.
- `model.normalized` MUST be `1`, and the reader MUST refuse an index declaring
  `0`. Normalising at read time is the alternative and is worse: it is the
  consumer computing vectors, and it would let an index whose geometry the
  producer never established be scored as though it had been.
- The reader MUST refuse a `metric` it does not compute. Reporting a metric
  through `Retrieval` while computing another one is the misdescribed evidence
  this proposal exists to remove.
- `verify` MUST check that every stored vector is unit-length within tolerance,
  for any index rather than only the committed fixture.

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
hql-semantics build   --vault DIR --out PATH [--offline]
                      [--provider local|http] [--model NAME] [--revision REV]
                      [--endpoint URL] [--api-key-env VAR] [--dimensions N]
                      [--query TEXT]... [--built-at STAMP]
hql-semantics verify  --vault DIR --index PATH    # exit 1 when stale
hql-semantics inspect --index PATH
```

- Default model `sentence-transformers/all-MiniLM-L6-v2`, `384` dimensions,
  under the default `local` provider.
- The model **revision** MUST be pinned, not only its name.
- Vectors MUST be L2-normalised, and `model.normalized` MUST record it.
- `--built-at` pins the build stamp, so rebuilding an unchanged vault produces a
  byte-identical file. A committed fixture that changes on every rebuild is a
  diff nobody can review.
- Every flag above overrides the matching `[semantics.embedder]` field, so a
  vault records the embedder it is indexed by and a one-off build need not edit
  the vault to change it.

#### Embedders

Exactly two, because they are the two shapes a model can be reached in, not the
two vendors anyone happens to use.

| Provider | The model runs | Reached by | Needs |
| --- | --- | --- | --- |
| `local` | in the producer's process | `transformers` and `torch` | the checkpoint in the local cache |
| `http` | anywhere else | an OpenAI-shaped `POST /v1/embeddings` | `endpoint`, `model`, `revision` |

`http` speaks one wire format — `{"input": [...], "model": "..."}` answered with
`{"data": [{"index": N, "embedding": [...]}]}` — which a hosted provider and a
model server on `localhost` both implement. Naming the transport rather than the
vendor is what keeps that one code path one code path.

- `http` MUST use only the standard library to make the request. A hosted
  embedder is not a reason for the producer to grow a dependency, and a
  contributor indexing through `http` MUST NOT need `torch` installed at all.
- `revision` MUST be supplied for `http`, and the producer MUST refuse without
  it. A hosted endpoint offers nothing to derive it from, and `Retrieval.revision`
  claims to pin the numbers a score is. Pushing that onto whoever configures the
  vault is the honest placement: an unpinnable claim is better refused than
  invented.
- The response MUST be ordered by its `index` field rather than trusting arrival
  order, MUST be refused if it holds a different number of vectors than were
  asked for, and MUST be refused if a width disagrees with `dimensions` or with
  the vectors already received.
- There is no embeddings endpoint at Anthropic to configure: Claude models are
  text-in, text-out. `http` covers OpenAI, Voyage, Cohere and a local
  `llama.cpp`, `vLLM`, Ollama or text-embeddings-inference server.

#### `--offline`

`--offline` MUST refuse the network, and with `provider = "http"` the network is
the whole mechanism, so the combination MUST be refused by name rather than
attempted and failed. What CI builds, CI builds locally.

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

A configurable embedder adds a credential, and a credential adds two rules.

- **A key lives in the environment, never in the vault.** `hql.toml` is
  committed. Configuration names the variable holding the key; the producer reads
  `os.environ`. A literal `api_key` in `[semantics.embedder]` MUST be refused,
  and the refusal MUST name `api_key_env`, because someone who wrote a key there
  needs to be told where it goes, not merely that it is wrong.
- **A missing variable MUST fail by name** before any card is read, so the
  failure is the unset variable rather than an HTTP 401 a hundred requests later.
- The key MUST NOT be written to stdout, stderr, or the index. `inspect` prints
  what an index says about itself, and an index holds no credential.
- An `http` embedder sends card text to whatever `endpoint` names. That is the
  point of it, and it is a disclosure a vault's owner MUST be making knowingly —
  which is a reason for the embedder to be declared in the vault, in a file that
  is reviewed, rather than to be a flag somebody passes once.

## Deployment / Activation

1. land [HQL-0001](../HQL-0001/README.md) first, registry and all
2. add the birds corpus, with `lexical` still in place
3. add `contrib/semantics/` and commit a generated index
4. rename the existing matcher to `lexical`
5. add the `semantic` extension reading the index
6. only then remove any remaining assumption that `semantic` needs no index

### What each side may depend on

The dependency runs one way, and this is what keeps the model out of the binary
a real property rather than an intention.

The **producer** may assume the HyperMarkDown CLI is installed: working with
`HmdCard`s is the default case, `hmd` owns root discovery and `.hmd/config.toml`,
and reimplementing that walk here would be a second answer to a settled
question. A vault is a directory with `.hmd/` in it; the config file inside is
optional and its defaults are assumed where it is absent, the directory is not.

The **consumer** may assume nothing but the index file. No Python, no `hmd`, no
model, no network: `cargo test` runs against the committed index, and a vault
that ships an index needs none of the producer's dependencies to be queried.

Wiring the producer to `hmd` is
[issue 0011](../../issues/0011-wire-the-semantic-search-toolchain.md); the chain
a query depends on is stated in
[the semantic-search requirements](../../models/requirements/semantic-search.md).

## Reference Implementation

- `tests/fixtures/birds/` — at least 40 documents: order, family and species
  cards; at least four relation cards, one with endpoints that deliberately do
  not resolve; `metadata.status` on a subset; `[[links]]` including one forward
  link to a document that does not exist
- `tests/fixtures/birds/.hql/index.sqlite` — generated, committed, about 136 KB
- `contrib/semantics/` — `cli.py`, `vault.py`, `embed.py`, `store.py`, `tests/`
- `src/extensions/lexical.rs` — today's `src/search.rs`, renamed
- `src/extensions/semantic.rs` — the SQLite reader and the scorer
- `src/search.rs` — what a retrieval is; `Retrieval` gains `revision`, and
  nothing here computes a score

## Test Plan

The articles MUST be written so the difference is measurable. Describe owls as
"nocturnal" and "after dark" and never as hunting "at night"; then
`lexical("night hunting birds")` finds nothing useful and `semantic` finds the
owls. At least **three** such pairs, asserted by card name. If these are weak,
the rest of the proposal does not compensate.

Two rules make the pairs measure what they claim to.

- An article MUST satisfy every part of its query in other words. An owl that is
  nocturnal but never described as a predator answers half the question, and
  half an answer loses to a decoy that answers the other half.
- A query MUST carry no function words. `lexical` tokenises `that` and `at` like
  any other word, so a query containing them ranks by how often a document
  happens to write `that`, which is evidence of nothing and makes the pair
  measure noise. The queries are therefore `night hunting birds`,
  `birds swimming underwater catching fish` and `birds weaving hanging nests`.

Each group's articles MUST have decoys: documents that do write the query's
words while meaning something else — herons that catch fish from above the
surface without entering it, a blackbird whose nest of grass is wedged into a
hedge rather than slung from a branch.

Unit tests MUST include:

- the golden text test: Python and Rust agree byte for byte on every card. From
  the Rust side this is the absence of a stale-card warning over the whole
  corpus, which needs no Python and so runs in `cargo test`.
- a truncated `embedding` blob rejected, not read past
- an unknown `schema_version` refused with the exact reason `unknown index schema`
- a stale `content_hash` producing a warning and still ranking
- a card absent from the index absent from the ranking, with the count warned
- a query with no stored vector failing with the command that would add one
- an index whose `index_meta.vault` names another corpus refused
- a configured index path that climbs out of the vault refused
- an index declaring `normalized = 0` refused, and one declaring a metric the
  reader does not compute refused
- `[semantics.embedder]` in a vault's `hql.toml` neither read nor refused by the
  binary, and a `Retrieval` reporting the index's model rather than that table's

The embedder's unit tests are unit tests: no network, no model download, and no
`torch`. A fake transport answers the HTTP provider and a fake model answers the
indexing path, so what is under test is the producer's own arithmetic and
refusals rather than a provider's availability.

- configuration parsed from `hql.toml`: an absent table meaning `local`, every
  field read, unknown keys and unknown providers refused by name
- a literal `api_key` refused, naming `api_key_env`
- `http` without `revision`, and without `endpoint`, each refused
- `--offline` with `provider = "http"` refused
- an unset `api_key_env` variable refused, naming the variable, before any
  request is made
- flag-over-file precedence, field by field
- the request a fake transport receives: the endpoint, the model, the batch, and
  a bearer header present only when a key was configured
- a response out of `index` order reordered; a short response, a wrong width, and
  a width disagreeing with `dimensions` each refused
- **the arithmetic**: a normalised pair whose dot product equals their cosine to
  tolerance; a non-unit vector normalised to unit length; a truncated prefix of a
  unit vector renormalised rather than stored short-of-unit; a zero vector
  refused rather than divided by
- an index built end to end from a fake model, read back, and every vector
  unit-length — the same check `verify` applies

Integration tests MUST include:

- the three paraphrase pairs
- `semantic` with no configured index failing with a name error naming
  `hql-semantics build`
- `cargo test` passing on a machine with no Python
- a vault carrying a full `[semantics.embedder]` table still answering, with its
  provenance taken from the index

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
- 2026-09-18: the schema grows a `query` table. Without it the proposal is not
  implementable: it gives the consumer no model and then asks it to score a
  query. The producer embeds queries, and an unknown query fails by name
- 2026-09-18: the embedded text is the *authored* header, because
  `Document::text()` was including the derived `path` and so scoring a card on
  its filename
- 2026-09-19: `name`, `path` and `format` became fields of `Doc` rather than
  header entries, so the producer filters only `title`; a key of that name in a
  header is an ordinary authored entry and is embedded
- 2026-09-19: what each side may depend on is written down. The producer may
  assume `hmd`; the consumer may assume only the index
- 2026-09-27: the embedder becomes the vault's choice. `[semantics.embedder]`
  declares one of two providers — `local`, a checkpoint in the producer's
  process, or `http`, an OpenAI-shaped endpoint covering a hosted provider and a
  local model server alike — with flags overriding it field by field, keys named
  as environment variables rather than written into a committed file, and
  `revision` required of `http` because a hosted endpoint offers nothing to
  derive it from. The producer gains no dependency: `http` uses the standard
  library, and indexing through it needs no `torch`. The consumer gains nothing
  at all, which is the point — it still has no model, and query-time embedding
  stays deferred
- 2026-09-27: the scorer's precondition becomes a checked one. `dot` is cosine
  only for unit-length vectors, and the reader was selecting neither
  `model.normalized` nor comparing `metric` against what it computes — an
  assumption that held only because one code path wrote every index. The reader
  now refuses `normalized = 0` and a metric it does not compute, the producer
  normalises whatever an embedder returns, and `verify` checks unit length for
  any index. Truncated embeddings are why this could not stay implicit: a prefix
  of a unit vector is not a unit vector
