# Changelog

Notable changes to HQL, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow
[SemVer 2](https://semver.org/spec/v2.0.0.html).

**Experimental.** While the major version is `0`, the language and the CLI may
change in any release, and a minor bump may break a program that ran before.

## [Unreleased]

## [0.1.0] — 2026-09-27

The first release: a typed expression core, a vault of Markdown and
HyperMarkDown documents, pipelines over its cards, two named retrievals, and
graph traversal that records why each node is present.

### Language

- A typed expression core — scalars, collection literals, lambdas, field access
  and pipelines — checked before it is evaluated.
- `type` declarations parsed and checked, so `std/` is read rather than assumed:
  `core`, `collections`, `doc`, `graph`, `knowledge` and `retrieval`.
- `Scalar` as a type, `Data` as a union, and a collection vocabulary settled on
  order: a sequence has one, a set does not, and `take` needs elements that
  carry their own.
- `:` as the subtype operator, replacing `<:`.
- A collection literal evaluates at its checked element type, and an unrelated
  tree is held as `Data`.
- `Hit` and `Ranking` carry the retrieval that produced them — query, index,
  model, model revision, metric, and whether the search was approximate.

### Retrieval and graph

- `lexical`, which matches shared spellings offline with nothing to build.
- `semantic`, which scores against vectors computed ahead of time;
  [`contrib/semantics/`](contrib/semantics/README.md) produces the index and the
  binary only reads it.
- Which model fills that index is the vault's choice. `[semantics.embedder]` in
  `hql.toml` declares one of two providers — `local`, a checkpoint in the
  producer's own process, or `http`, an OpenAI-shaped `/v1/embeddings` endpoint
  covering a hosted provider and a model server on `localhost` alike — and every
  `build` flag overrides it field by field. A key is named as an environment
  variable and never written into the committed file, and `http` needs an
  explicit `revision`, because an endpoint offers nothing to derive the pin a
  score claims. The producer gains no dependency: `http` uses the standard
  library, and indexing through one needs no `torch`.
- Scoring checks the precondition it rests on. A dot product is cosine only for
  unit-length vectors, so the producer normalises whatever an embedder returned,
  the index refuses to be written holding a vector that is not unit-length, and
  the reader refuses an index declaring `normalized = 0` or a metric it does not
  compute. Truncated embeddings are why this cannot be implicit: a prefix of a
  unit vector is not a unit vector.
- `expand`, which traverses outwards and records the reason each node is
  present, so a graph of matches and their neighbours does not claim that all of
  them matched.
- Links traverse as uplinks and downlinks.

### The command line

- `eval`, `check`, `run`, `repl`, `render`, `builtins` and `config`. `-` reads
  standard input wherever a file is taken.
- `--format json` emits the type, the value and the report queue for another
  program.
- `hql render --write` runs the `hql#eval` blocks in a document and writes each
  answer beneath it as an `hql#result` block, replacing the previous one.
- Reporting yields a value *and* a queue: `--report strict` stops at the first
  error and is the default, `--report collect` reports every error findable. The
  mode comes from the query, then the vault's `hql.toml`, then `$HQL_REPORT`,
  then the default.
- Exit `0` on success, `1` on a language or I/O failure, `2` on a usage error.
  Diagnostics carry zero-based UTF-8 byte ranges and render with a line, a
  column and a caret.

### Extensions and execution

- An extension mechanism with declared contracts, and a checked execution tree
  with structural step contracts.
- MapReduce pipeline stages with pluggable execution backends.

### Corpora

- `tests/fixtures/birds/`: fifty-six encyclopedia articles with an embedding
  index committed beside them.
- [`examples/birds/`](examples/birds/README.md): linked HyperMarkDown cards with
  runnable collection queries, including intentional errors for duplicate keys
  and non-orderable sorted-map keys.

[Unreleased]: https://github.com/ewiger/hql/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ewiger/hql/releases/tag/v0.1.0
