# Changelog

Notable changes to HQL, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow
[SemVer 2](https://semver.org/spec/v2.0.0.html).

The language continues to be in active development.

## [Unreleased]

### Added

- **Cells.** A cell's info string takes `id=<name>`, `replace`, `after` and
  `hidden`. A cell with an id exports its value to every other cell of the
  document and keeps its own bindings. The document is checked as a whole and
  evaluated in dependency order, document order breaking ties; a cycle and a
  duplicate id are errors naming the cells, and a cell waiting on a failed one
  is skipped. An output under `after` records the cell's id, a hash of its code
  and of the cells it reads, and its status. `hql render --cell <id>` runs one
  cell and what it reads; `--envelope json` prints the document and a record
  per cell.

### Fixed

- A cell's reports point at its place in the document, not at a line of the
  cell read as though it were the document.

## [0.2.0] — 2026-10-19

Functions as values, streams and generated sources, aggregation, and
presentation formats. `cards` becomes a lazy sequence, and presentation moves
from terminal presenters to formats a program can keep computing with.

### Changed — breaking

- `cards` is the standard-library binding `cards = vault.cards`, a lazy
  `Seq<Card>` in the order a sorted walk of the vault reaches each card: the
  entries of each directory in name order, depth first. It was a `Set<Card>`.
  A pipeline over `cards` is a sequence: `filter`, `sort` and `map` keep a
  `Seq`. The host seeds `vault`, of type `Vault`; without a vault, neither
  `vault` nor `cards` is bound.
- The `present` extension, the `Presentation` type and the `table` step are
  gone. `text`, `markdown` and `json` are shorthands of `format`, and their
  results are text a later step reads. `table` is an unknown step whose
  diagnostic names `Table`, which arrives with record values.
- The `graph` step is the `Graph` constructor: `Graph(xs)` or `xs | Graph`.
  `cards | graph` is refused naming `Graph`. `expand(depth = 0)` is refused.
- `--format text|json` is `--envelope text|json`, the CLI's output format.
- A hit's retrieved value is `h.value`; reading `h.card` names `value`.
- `map` over a `Set` returns its image, a `Set`, so equal results collapse.
- `typed` is `of_type`.
- Plain text writes a value as its own rendering, `[a, b]`, not an aligned
  table. `--to markdown` or `format(Markdown, renderer = "pipe")` gives a table.
- A graph's fields are `nodes : Set<Card>` and `edges : Set<Edge>`.
- `std/` states the fields the binary reads: `Hit` and `Ranking` hold
  `score : Float`, `query : String` and `retrieval : Data`; `Doc` declares
  `title` and `body : String`; `Card` declares `kind`. `Score`, `Query`,
  `Retrieval`, `Content` and `HmdContent` are gone.

### Added

- **Function values.** Function types written with arrows, `Int -> Int`;
  `fn` declarations, recursive and variadic (`xs: ...Int`); lambdas as values,
  typed by annotation or by their binding; functions passed and returned.
  Parameters are contravariant and results covariant.
- **Partial application with a hole.** `add(10, _)` is `Int -> Int`: each hole
  takes the type of the parameter it fills.
- **Branching and absence.** `match` with value, type and `some`/`none` arms,
  checked for exhaustiveness; `value given condition otherwise other`; `some`,
  `none` and `somes`.
- **Asking what a value is.** `is(T)` and `of_type(T)`; a `filter` by `is`
  narrows the element type; a `Data` leaf reads as `Int`, `String` and the
  rest, so a metadata field can be summed.
- **Aggregation.** `group`, `reduce`, `sum`, `min` and `max` over finite
  inputs. One signature per name, printed by `hql builtins` one line per
  overload.
- **`tee`**, which binds a side branch of a pipeline and passes its input on.
- **Streams.** `Stream<T>` is a type, and `Seq<T>` a stream that ends.
  `map`, `filter`, `take` and the new `drop` run lazily over a stream; a step
  that needs its whole input refuses one, naming `take(n)`. `collect(n)`,
  `emit` and `flat_map` batch and unbatch; `List(xs)` refuses a stream and a
  set.
- **Generated sources.** A `fn` with a block body yields its values, directly
  or in a `for` loop, and returns `Seq<T>` or `Stream<T>`. Calling it runs
  nothing; `take` ends a stream that would not end by itself.
- **Constructors as pipeline steps.** `xs | Set` is `Set(xs)`.
- **Presentation formats.** `Text`, `Markdown`, `Hmd`, `Json`, `Yaml`, `Csv`,
  `D2` and `Mermaid` are subtypes of `String`. `format(x, F)` with `limit` and
  `renderer`, and the shorthands `text`, `markdown`, `hmd`, `json`, `yaml`,
  `csv`, `d2` and `mermaid`. Graph renderers write adjacency text, node-link
  JSON, D2 and Mermaid, each node saying why it is present. A vault chooses
  renderers under `[format.Type]` in `hql.toml`. Over a stream, `format` gives
  a stream of pieces.
- **The presentation context.** `--to <format>` writes a result the program did
  not format; `--to none` evaluates and writes nothing. `hql render` writes
  each cell as `hmd`. A stream result is written piece by piece as it arrives,
  and ends when its reader stops reading.
- `hql doctor` checks the semantic-search chain — vault, `hql.toml`, index,
  model, query vectors, card coverage — and names the first missing part.
- `hql render` builds a Markdown document from its cells. `--emit replace`, the
  default, writes each cell's output in its place; `--emit after` keeps the
  cell and writes the output after it between `<!-- hql:output -->` markers.
  `--write` takes the output path, and `--in-place` rewrites the source under
  `--emit after`.
- Scalar operators: `<`, `<=`, `>`, `>=` over values of one orderable type;
  `and`, `or`, `not` over `Bool`, short-circuiting; `-`, `*`, `/`, `%` over
  `Int` or `Float`, with truncating `Int` division. Division by zero and `Int`
  overflow are runtime errors. A comparison does not chain.
- The `debug` step: an identity step that records its input, a label and its
  trace in the report log at severity `debug` or `info`. `--report-level`, the
  vault's `[report] level` and `$HQL_REPORT_LEVEL` set which reports the text
  output shows; JSON output carries all of them.
- `std/` is embedded in the binary, and a built-in record's fields are read
  from its declaration.
- `Ranking<T>` is a declared subtype of `Seq<Hit<T>>` over any element type.
- Corpora: `examples/functions/`, `examples/streams/` and
  `examples/mapreduce/`, every case checked by `tests/corpus.rs`.

### Fixed

- `cards = [1, 2]` followed by `cards` evaluated to the vault's cards and
  ignored the binding without a report.
- `1 - 2` was a syntax error: a minus after an operand is subtraction, and only
  a minus before a digit with no operand ahead of it is a literal's sign.
- A step written inside a pipeline reports its own span, not the whole
  pipeline's.
- A wikilink's display text is not part of its target.
- Equality looks through subtyping, so `Json` text compares with a string.
- CSV lays a list of records out in one column per key.

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

[Unreleased]: https://github.com/ewiger/hql/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ewiger/hql/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ewiger/hql/releases/tag/v0.1.0
