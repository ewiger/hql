# HQL-0058: AI memory: fused rankings, a written index, and `recall`

**Status**: drafted
**Created**: 2026-11-13
**Source**: [HQL-0002](../HQL-0002/README.md), whose non-goals on the write
path, chunking and hybrid retrieval this record takes up;
[HQL-0022](../HQL-0022/README.md), whose Memory and Query-time embedding
sections it extends; the "Open" list of the
[semantic search](../../wiki/hql/semantic-search.hmd) card

## Companion notes

- [`analysis.prompt.md`](analysis.prompt.md): a prompt for analysing how
  search, storage and memory are wired, against the code as it stands. Its
  report is the evidence this record is revised from.

## Abstract

AI memory is a loop over one corpus: cards are retrieved, a run reads them, the
run's result is written as a card, and that card is retrieved later. This record
closes the loop in three places. **Search**: every kind of search over a corpus
is a `Ranking`, and `fuse` combines rankings by rank into one ranking whose
`retrieval` holds the retrieval of every part. `rank` makes a ranking from an
attribute and `near` from graph distance, so text, meaning, front matter and
links are combined by one step. `filter` stays the exact, boolean restriction
of the corpus and is not a signal. **Storage**: the index a vault names becomes
a store the binary writes. `embed` embeds cards through the index's own `http`
embedder, under the modes and recordings of HQL-0022, and index schema `2` adds
a `chunk` table whose unit is a section. **Memory**: a new standard module,
`ai.memory`, declares `memories`, `recall`, `sources` and `stale`. `remember`
records what a memory was derived from and embeds the card it writes. A
memory's `depth` is the number of derivation steps between it and cards no run
wrote, so an abstraction is a memory whose sources are memories, and no step
is added to produce one.

## Motivation

- The four kinds of search a program has today compose only in sequence.
  `filter` restricts by front matter, `lexical` and `semantic` each rank,
  `expand` traverses. A program can filter and then rank, or rank and then
  traverse. It cannot state that a card found by both rankings outranks a card
  found by one, or that a recent card outranks an old one of the same
  similarity, without discarding the `retrieval` of every hit and sorting
  records by hand.
- A fused score has no honest `retrieval` today. Scores of two models are not
  comparable numbers, and `Hit.retrieval` describes one retrieval. The semantic
  search card lists this as open.
- The index has a reader and no writer in HQL. It is produced by
  `contrib/semantics/`, outside the binary. A card written by `remember` is
  absent from the index until that tool runs, so `semantic` skips it with the
  coverage warning: a vault cannot retrieve by meaning what it has just
  remembered.
- A card is the only unit a vector exists for. A long card's vector is an
  average over everything it says, and the passage a query is about cannot be
  named by a hit.
- `remember` records the run that wrote a card and nothing the run read. What a
  remembered statement rests on is recoverable only through the recording, and
  a memory derived from other memories is indistinguishable from one derived
  from authored cards.

## Goals

- One step combines any number of rankings of one corpus, and every part's
  `retrieval` is recoverable from every fused hit.
- A card written by `remember` is retrievable by `semantic` in the next run of
  the program, with no tool outside `hql`.
- What a memory was derived from, and whether that has changed since, is read
  from the memory card.
- `cargo test` stays offline and deterministic, and `hql check` writes nothing.

## Non-goals

- **No approximate search.** Scoring stays exact brute force and
  `approximate` stays `false`. A store that answers approximately, such as a
  `postgres` endpoint with a vector column, is a later record; `fuse` already
  reports `approximate` as the disjunction of its parts.
- **No second index.** A vault names one index and the memory directory is
  inside the vault, so memory cards and authored cards share it.
- **No score-weighted fusion.** `fuse` reads ranks, never scores.
- **No automatic memory, no consolidation step, no eviction.** A run never
  writes memory by itself, as HQL-0022 states. An abstraction is written by a
  `remember` the program wrote. A superseded memory stays a card.
- **No `recall` over `Knowledge`.** `recall` here takes a set of cards. A
  concept with no card is not in a set of cards, so nothing is skipped
  silently. The overload over `Knowledge` waits on that type.
- **No embedder in the binary.** A `local` embedder stays the producer's.
  `embed` and query-time embedding require the `http` embedder.

## Specification

Depends on [HQL-0002](../HQL-0002/README.md) for `Hit`, `Ranking` and the
index, [HQL-0022](../HQL-0022/README.md) for runs, modes and recordings, and
[HQL-0019](../HQL-0019/README.md) for graphs.

### 1. One corpus, five operations

The loop is stated in the types that already exist.

```text
Set<Card>           | filter(p)                   -> Set<Card>       restriction: exact, boolean
Set<Card>           | lexical(q) | semantic(q)    -> Ranking<Card>   a signal from text
Set<Card>           | rank(by) | near(seeds)      -> Ranking<Card>   a signal from front matter or links
List<Ranking<Card>> | fuse                        -> Ranking<Card>   signals combined, parts retained
Ranking<Card>       | Context | run | remember    -> Card            written, embedded, in the corpus next run
```

- A **signal** is a step from a corpus to a `Ranking`. A score is evidence
  under one signal and is never compared with a score under another.
- A **restriction** is a `filter`. It has no score and changes which cards a
  signal ranks.
- `filter` before a signal and `filter` after it give the same hits for
  `lexical` and `semantic`, because each card's score is independent of the
  other cards. They differ for `fuse`, `rank` and `near`, whose scores are
  positions. A restriction that is to affect ranks MUST precede the signal.

### 2. `rank` and `near`

Two signals are added to the `retrieval` module of `std`. Both are `Pure`.

```hql
fn rank<K : Orderable>(xs: Set<Card>, by: Card => Option<K>, *, name: String, descending: Bool = true) : Ranking<Card>
fn near(xs: Set<Card>, seeds: Set<Card>, *, depth: Int = 2) : Ranking<Card>
```

```text
rank:  position p = 1.. in the order of `by`, ties by card name
       score = 1 / p              a card whose key is absent is not a hit
       retrieval = {model: "hql.rank.v1", revision: <crate version>, metric: "position",
                    key: name, approximate: false, query: ""}

near:  d = the least number of links between the card and any seed, in either direction
       score = 1 / (1 + d)        a card with d > depth, or with no path, is not a hit
       retrieval = {model: "hql.near.v1", revision: <crate version>, metric: "link-distance",
                    seeds: [names], depth: depth, approximate: false, query: ""}
```

- `name` is required on `rank`, because a lambda has no name and a
  `retrieval` that cannot say which attribute ordered the hits is not
  evidence.
- `depth` MUST be between `1` and `8`.

### 3. `fuse`

`fuse` combines rankings of one corpus by reciprocal rank.

```hql
fn fuse(parts: List<Ranking<Card>>, *, weights: List<Float> = [], k: Int = 60) : Ranking<Card>
```

```text
score(c)  = sum over parts i that hold c of  w_i / (k + rank_i(c))     rank is 1-based
order     = descending score, ties by card name
query     = the distinct non-empty queries of the parts, in part order, joined by " | "
retrieval = {model: "hql.rrf.v1", revision: <crate version>, metric: "reciprocal-rank",
             k: k, approximate: <any part approximate>,
             parts: [each part's retrieval, with its weight]}
hit.retrieval = retrieval, plus  ranks: [rank_i(c) or absent, per part]
```

- A card is identified by its name. A card held by no part is not a hit.
- `weights` MUST be empty, meaning `1.0` for each part, or as long as `parts`
  with every weight greater than `0`. `k` MUST be at least `1`. `parts` MUST
  NOT be empty.
- The result is a `Ranking`, so a fused ranking is a part of another `fuse`,
  an input of `Context` and a receiver of `take`, `filter` and `expand` with
  no further rule.
- Ranks are read and scores are not, because a rank is comparable across
  models and a score is not. This is what lets `retrieval` stay true: it names
  the fusion and every part, and each hit holds its rank under each part.

```hql
import lexical
import semantic

q = "which unread book suits a reading club"
shelf = cards | filter(c => c.metadata.category == "book" and c.metadata.status == "todo")

fuse([
    shelf | semantic(q),
    shelf | lexical(q),
    shelf | rank(by = c => c.metadata.added, name = "added"),
    shelf | near(@[[club]] | Set),
])
| take(5)
```

### 4. The index is written by `embed`

The `semantic` module gains one `Effectful` step.

```hql
effectful fn embed(xs: Set<Card>) : EmbedReport

type EmbedReport {
    embedded : List<String>   // names whose vectors were written
    current  : List<String>   // names whose stored hash already matched
    model    : String
    revision : String
}
```

- `embed` MUST require `[semantics.embedder]` with `provider = "http"`, whose
  `model` and `revision` MUST equal the index's, as HQL-0022 requires of a
  query. With a `local` embedder it MUST fail naming `hql-semantics build`.
- For each card whose `content_hash` in the index is absent or differs from
  the hash of `Document::text()`, it sends the text to the embedder,
  normalises the vector to unit length, checks its `dimensions`, and writes
  the `card` row and the card's `chunk` rows in one transaction.
- Requests go through the recordings and the `--ai` mode of HQL-0022.

| Mode | `embed` |
| --- | --- |
| `replay` (default) | succeeds only when every card is `current`; otherwise fails naming `--ai record` |
| `record` | embeds from the recording when present, otherwise sends and records; writes |
| `live` | sends, never records; writes |

- An index that does not exist is created by `embed` under `record` and
  `live`, with `index_meta.vault` naming the loaded vault.
- `hql check` MUST NOT send a request or write the index.
- `semantic` is unchanged: it reads, and its staleness and coverage warnings
  are what a program reads to learn that `embed` is due.

### 5. Index schema `2`: chunks

A chunk is a section of a card. Schema `2` adds one table and changes none of
the four that schema `1` declares.

```sql
CREATE TABLE chunk (
    card         TEXT NOT NULL REFERENCES card(name),
    anchor       TEXT NOT NULL,       -- the section's heading path, "/"-joined
    content_hash TEXT NOT NULL,       -- sha256 of the chunk text, lowercase hex
    embedding    BLOB NOT NULL,       -- f32 little-endian, `dimensions` of them
    PRIMARY KEY (card, anchor)
);
```

```text
text(chunk)  = card title | section heading | section body      joined by " "
score(card)  = max(card vector · q, every chunk vector of the card · q)
hit.retrieval.chunk = the anchor of the best chunk; absent when the card vector scored best
```

- The reader MUST accept schema `1` and `2` and refuse any other. Over a
  schema `1` index, a card's score is its card vector's, as today.
- A card with one section or none has no chunk rows.
- The hit's value stays the card, so `semantic` stays
  `Set<Card> -> Ranking<Card>` and a passage is named by `retrieval.chunk`.
- The producer in `contrib/semantics/` and `embed` MUST write the same rows
  for the same card, and the golden test of HQL-0002 extends to chunk text.

### 6. The `ai.memory` module

`ai.memory`, in `lib/std/ai/memory/mod.hql`, is provided by the `ai`
extension and imported with `import ai`.

```hql
memories : Set<Card>                       // the cards under `[ai.memory] dir`

effectful fn remember<T>(x: T, name: String, *, sources: List<Card> = [], supersedes: Option<String> = none) : Card

fn sources(card: Card) : List<Card>
fn stale(card: Card) : Bool
fn recall(xs: Set<Card>, query: String) : Ranking<Card>
```

`remember` moves here from `ai.agents` and gains two arguments. The header of
the card it writes is:

```yaml
memory:
  run: 5e10ae23…            # as HQL-0022: the run, its agent and model
  agent: librarian
  model: claude-opus-5-5
  depth: 2
  sources:                  # name -> sha256 of Document::text() when written
    next-meeting-pick: 9c1f…
    the-common-reader: 44ab…
  supersedes: club-taste    # optional
```

- Over a `Run`, `sources` defaults to the cards named by `provenance.name` in
  the items of `r.prompt.context`, in context order and without repeats. Over
  a `String` it defaults to empty.
- `depth` MUST be `1 + max(depth of each source)`, where a card with no
  `memory` table has depth `0`, and `1` when there are no sources.
- `supersedes` MUST name a card in the memory directory. The named card is
  not modified.
- Under `record` and `live`, when the vault declares an `http` embedder,
  `remember` MUST embed the card it wrote, as `embed` does. With no such
  embedder it writes the card only, and `semantic` reports the card through
  its coverage warning.
- The idempotence, path and mode rules of HQL-0022's Memory section are
  unchanged, and the header above is part of the content compared.
- `sources(card)` is the cards of the vault named by `memory.sources`, in
  written order. A name that no longer resolves is a warning naming it.
- `stale(card)` is `true` when a source is absent, when a source's current
  hash differs from the recorded one, or when a source is itself stale.

### 7. `recall`

`recall` is declared in HQL, over the steps above.

```hql
fn recall(xs: Set<Card>, query: String) : Ranking<Card> = {
    superseded = xs | map(c => c.metadata.memory.supersedes) | flatten | Set
    live = xs | filter(c => not (c.name in superseded))
    fuse([live | semantic(query), live | lexical(query)])
}
```

- With no index, `recall` fails as `semantic` does. It MUST NOT answer from
  `lexical` alone.
- A hit whose value is a `RelationCard` MUST be replaced by its relation's
  source and target cards at that hit's position, each once, because a
  relation card contributes an edge and is not a seed.
- `recall` takes its corpus as an argument. `memories | recall(q)` reads
  memory, `cards | recall(q)` reads the vault, and a `filter` before it reads
  a part of either:

```hql
import ai

memories
| filter(c => c.metadata.memory.depth >= 2 and not stale(c))
| recall("what the club likes to read")
| take(3)
| Context("memories")
```

### 8. Memory in a context

A reader of a prompt and a reader of a trace can tell remembered text from
authored text.

- `Context` over a `Card` whose header holds a `memory` table MUST contribute
  `provenance.memory`, holding `run`, `depth` and whether the card is stale,
  beside `provenance.name` and `provenance.path`.
- In `Prompt.rendered`, such an item's source line MUST read
  `Source: <name> (memory, depth <n>)`, and `Source: <name> (memory, depth
  <n>, stale)` for a stale one.

### 9. Abstraction is derivation

No step produces an abstraction. A memory whose sources are memories has a
depth one greater than theirs.

```hql
import ai

memories
| filter(c => c.metadata.memory.depth == 1)
| recall("books the club picked")
| take(8)
| Context("picks")
| run(@[[librarian]] | Agent)
| remember("club-taste")
```

```text
depth 0   a card no run wrote
depth 1   a memory whose sources are all depth 0, or that has none
depth n   a memory whose deepest source has depth n - 1
```

Editing a depth-0 card makes every memory above it stale, through `stale`,
which is the rule a cell's hash already applies to the cells it reads.

## Backwards Compatibility

- Additive in `retrieval`: `rank`, `near` and `fuse` are new names.
- `remember` moves from `ai.agents` to `ai.memory`. `import ai` binds both, so
  a program that imports `ai` is unchanged; a program that imports
  `ai.agents` alone and calls `remember` MUST import `ai.memory`.
- A memory card written before this record has no `depth` and no `sources`,
  and reads as depth `1` with no sources.
- A schema `1` index stays readable. The first `embed` over it migrates it to
  schema `2` in one transaction.

## Security Considerations

- **A memory is model output, and it re-enters a context.** Text a model wrote
  in one run is read by a model in a later run. `provenance.memory` and the
  source line mark it in every prompt, tools stay pure, and `remember` stays
  uncallable by a model, so remembered text gains no authority an authored
  card lacks.
- **Disclosure.** `embed` and `remember` send card text to the embedder's
  endpoint. The endpoint is declared in a reviewed file and sending is
  authorised by `--ai`, as for a model request.
- **The index is written inside the vault.** The path MUST resolve inside the
  vault root for writing as it does for reading. A write MUST be one
  transaction, so an interrupted `embed` leaves the previous rows.
- **Resource bounds.** `fuse` accepts at most `16` parts. `near` visits at
  most `depth` links from each seed. One `embed` request holds at most `64`
  texts, and a chunk text is at most `65536` bytes; a longer section is
  embedded as its first `65536` bytes on a character boundary, with a warning
  naming the card and anchor.

## Deployment / Activation

1. `fuse`, `rank` and `near` in `retrieval`. They are pure and need no model
   and no index beyond the committed fixture.
2. `recall` over `fuse`, with `memories`, `sources` and `stale`, and the new
   `remember` header.
3. `embed` against schema `1`, under `replay` and a scripted embedder.
4. `remember` embedding the card it writes.
5. Schema `2` and chunks, in the producer and in `embed`, in one change.
6. The Learn chapters on retrieval and on agents inside documents, and the
   semantic search card, state what is built.

## Reference Implementation

- `lib/std/retrieval/mod.hql`: `rank`, `near`, `fuse`
- `lib/std/ai/memory/mod.hql`: `memories`, `remember`, `sources`, `stale`,
  `recall`
- `lib/search/`: `embed` and `EmbedReport` in the `semantic` module
- `src/extensions/search/`: the index writer, schema `2`, chunk scoring
- `src/extensions/ai/`: the `remember` header, `provenance.memory`, the
  source line of `rendered`
- `contrib/semantics/`: schema `2` and chunk rows
- `examples/`: the cases of this record, proposed first and promoted as each
  one checks

## Test Plan

Every test runs in `replay` against committed recordings or a scripted
embedder.

Unit tests MUST include:

- `fuse` of two rankings pinned score for score, a card held by one part
  only, ties ordered by name, and `retrieval.parts` holding both retrievals
- `fuse` refusing empty `parts`, a `weights` of the wrong length and `k = 0`
- `filter` before and after `fuse` giving different ranks, pinned
- `rank` dropping a card whose key is absent, and `near` dropping a card
  beyond `depth`
- `embed` under `replay` succeeding over a current index and failing, naming
  `--ai record`, over a stale one
- `embed` refusing a `local` embedder and a model or revision that differs
  from the index's
- a schema `1` index read unchanged, and a chunk outranking its card vector
  with `retrieval.chunk` set
- `remember` over a `Run` recording the context's cards as `sources`, `depth`
  computed over two levels, and `supersedes` naming an absent card refused
- `stale` true after a source's text changes, and through a stale source
- `recall` excluding a superseded card and replacing a `RelationCard` hit by
  its endpoints
- the `rendered` source line of a memory item pinned byte for byte

Integration tests MUST include:

- a document that remembers in one render and recalls the card by meaning in
  the next, from recordings, with byte-identical output on a third render
- `hql check` of that document with no credentials, no network and an
  unmodified index file

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-11-13: drafted
