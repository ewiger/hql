# HQL-0017: Presentation formats

- **Status**: accepted, implemented in part; superseded in one rule by [HQL-0024 part 7](../HQL-0024/parts/07-formats-and-codecs.md): a format is a `Format`, not a `String`, and `format(x, F)` gives a `String`
- **Created**: 2026-10-18
- **Source**: [presentation](../../wiki/hql/presentation.hmd),
  [issue 0017](../../issues/0017-graph-projection-and-view.md)

## Abstract

Presenting a result answers three separate questions, and each gets its own
mechanism. **What the value is** is a type: `Graph`, `Table` and `Tree` are
ordinary data types, built by their constructors, so the projection from cards
to a graph is `Graph(xs)` and a table is `Table(xs)`. **Which syntax it is
written in** is a **presentation format**, a subtype of `String` such as
`Markdown`, `Json` or `D2`. The operation `format(x, F)` turns any value into
text of format `F`; it is an ordinary operation whose result type is the
format, so a step after it is checked like any other. `markdown`, `json`, `yaml`, `csv`, `d2`, `mermaid`, `hmd` and
`text` are its shorthands, each a host step whose longhand `std/format.hql`
states as a `:=` line ([HQL-0015](../HQL-0015/README.md)). **How a type is written in a format** is
a **renderer**, chosen per type and format from configuration: a `Graph` in
`Json` is node-link JSON, in `Markdown` and `Hmd` a fenced D2 block. All of it lives in the `format` extension,
which replaces `present`. A result that is not yet formatted is formatted in
the **presentation context**, which the query, the command line or the host
sets. This record supersedes [HQL-0008](../HQL-0008/README.md), except its
section "Presenting into a document", which it restates as a presentation
context.

## Motivation

- The [presentation](../../wiki/hql/presentation.hmd) card lists `table`,
  `graph`, `json`, `markdown` and `text` side by side, but they answer
  different questions. `table` and `graph` name shapes of data; `json`,
  `markdown` and `text` name syntaxes. Treating them as one category leaves
  `graph` doing two jobs and makes `cards | graph | table` read as two
  renderings in a row.
- A presenter today returns an opaque `Presentation` that nothing may read.
  A program cannot measure, store or embed the text it produced, and the
  question of presenter arguments has no home.
- One shape needs different renderings in different places: a graph is an
  edge list in a terminal, node-link JSON for a program, a D2 block in a
  Markdown file and a live component in a HyperMarkDown page. The query should
  not change for each.
- `--format` on the command line chooses the envelope of the answer, not a
  data syntax, and there is no way to name an input format.

## Non-goals

- **No new rendering engine.** D2 and Mermaid are emitted as their own source
  text; HQL does not lay out or draw a graph.
- **No file effects.** Writing the formatted text to a file stays the host's,
  as it is today.

## Specification

### 1. Terms

- A **presentation format** is a type of text with a known syntax. In code and
  in `hql builtins` the extension and the operation are both `format`; in prose
  the term is "presentation format".
- A **renderer** writes values of one type in one format.
- The **presentation context** is the format a host applies to a result that
  is not yet formatted.

`Doc`'s field `format`, the dialect of a document's body, is read as `d.format`
and is not the operation; a field and an operation do not share a namespace.

### 2. Shape types

The shapes a renderer targets are data types, built by construction as
[HQL-0016](../HQL-0016/README.md) builds every conversion. The shape types
and the rules of each are stated in
[presentation](../../wiki/hql/presentation.hmd), under "Shape types".

| Type | Declared | Built by |
| --- | --- | --- |
| `Graph<N, E>` | `std/graph.hql` | `Graph(xs)`: the elements as nodes, the links among them as edges |
| `Table` | `std/format.hql` | `Table(xs, columns = [...])`: one row per record |
| `Tree<T>` | new | `Tree(root, children = …)`: a node and its subtrees |
| `Data` | `std/core.hql` | parsing, see "Reading a format" |
| `Doc` | `std/doc.hql` | the vault |

- `Graph(xs)` MUST be the projection from cards, hits or a ranking to a graph.
  `graph` is not an operation: it is the binding `graph = vault.graph`, the
  current vault's knowledge graph ([HQL-0019](../HQL-0019/README.md)). Used as
  a step, `cards | graph` MUST be refused with a diagnostic naming `Graph`.
- `expand(xs, depth)` grows a graph from its input. `depth` MUST be at least
  `1`, since `expand(depth = 0)` would be `Graph` under a second name.
- `Table(xs, columns = [...])` MUST build one row per element, each a
  record or a `Data` map, and `columns` MUST name the columns in the order
  every format writes them; without it, the columns are the first row's
  fields in the order the row holds them. Over a declared record, a column
  naming no field MUST be a type error.
- The step `table` MUST be an unknown operation whose diagnostic names `Table`.
- There is no `Presentation` type. A value keeps its type until it is
  formatted.

### 3. Format types

Each format is a subtype of `String`:

```hql
type Text     : String
type Markdown : String
type Hmd      : Markdown
type Json     : String
type Yaml     : String
type Csv      : String
type D2       : String
type Mermaid  : String
```

- A value of a format type MUST be text that parses in that format.
- `Hmd : Markdown` because every HyperMarkDown text is Markdown; a component a
  Markdown viewer cannot display still leaves valid Markdown around it.

### 4. `format` is the longhand

```hql
fn format<T, F : String>(x: T, as: Type<F>, limit: Int = 0, renderer: String = "") : F
```

- `format` MUST return text of format `F`, so the result type is known before
  the program runs.
- `format` MUST be pure and MUST NOT be terminal: a step after it is checked
  like any other.
- `limit`, when positive, MUST cap the top-level elements rendered (rows of a
  table, nodes of a graph, items of a list), and the renderer MUST mark the
  cut in the text.
- `renderer`, when given, MUST override the configured renderer for this call.
- A missing renderer for the type and format MUST be a check error naming
  both.
- Over a value already of a format type: `format(x: F, F)` MUST be an
  identity; a `Markdown` value formatted as `Hmd` widens and copies nothing;
  `D2`, `Mermaid`, `Json`, `Yaml` and `Csv` formatted as `Markdown` MUST
  become a fenced block with that language's info string.

### 5. The shorthands

The `format` extension defines one shorthand per format:

```hql
text     := format(Text)
markdown := format(Markdown)
hmd      := format(Hmd)
json     := format(Json)
yaml     := format(Yaml)
csv      := format(Csv)
d2       := format(D2)
mermaid  := format(Mermaid)
```

- A shorthand MUST fix the format and nothing else. An option is written in
  the longhand, `format(Markdown, limit = 50)`; a name is taken by the first
  declaration that claims it, so `markdown` has one signature.

### 6. Renderers and their configuration

A renderer is one of a format's encoders for one shape
([HQL-0024 part 7](../HQL-0024/parts/07-formats-and-codecs.md)). The
`format` extension registers the built-in renderers; another extension MAY
register more. A vault's `hql.toml` maps each type and format to a renderer,
over built-in defaults. The renderers, the encoder each format applies to a
`Graph` and a `Table` with no renderer named, and the `[format.<Type>]`
configuration are stated in [presentation](../../wiki/hql/presentation.hmd),
under "Renderers".

- A value whose type has no entry MUST fall back to the entry of its nearest
  supertype that has one, and then to the built-in encoder for its format.
- Each format MUST write a `Table` with its own encoder when no renderer is
  named, its columns in the table's order.
- A graph renderer MUST write only a `Graph` and a table renderer only a
  `Table`. Naming one for another value MUST be refused when the program is
  checked, where the value's type is known, and when the configuration is
  read.
- An unknown renderer name in the configuration MUST be reported when the
  configuration is read, not when a query reaches it.

### 7. The presentation context

A result that is not already formatted is formatted in the context, as if
the pipeline ended in `| format(<context>)`. Which context applies, first
match wins, and how a host places a result already formatted are stated in
[presentation](../../wiki/hql/presentation.hmd), under "The presentation
context".

- A result already formatted MUST be written as it is.
- `--to none` MUST evaluate the query and write no value.
- The flag that chooses the envelope, today `--format text|json`, MUST become
  `--envelope text|json`, so `--format` names nothing and `--to` names the
  presentation format.
- [Knowledge](../../wiki/hql/knowledge.hmd) MUST still have no default: a
  knowledge result in a context with no renderer for it is a check error
  naming a projection to write.

### 8. Reading a format

- `--from <format>` MUST name the format of standard input when it is a query
  source ([issue 0015](../../issues/0015-jq-parity-analysis.md)).
- Text is parsed by construction, the format named:
  `s | Data(Json)` parses a `Json` or `Yaml` value
  ([HQL-0018](../HQL-0018/README.md)); `s | Table` parsing a `Csv` value is
  not implemented. A plain `String` is parsed as the format `--from` names, or
  refused.

### 9. Streams

```hql
fn format<T, F : String>(xs: Stream<T>, as: Type<F>) : Stream<F>
```

- `format` over a `Stream<T>` that is not a `Seq<T>` MUST return a
  `Stream<F>`: one piece of format `F` per value, produced as the value
  arrives. A piece is a line in `Text`, a list item in `Markdown` and `Hmd`, a
  row in `Csv` with the header in the first piece, one value on one line in
  `Json`, and one document opening with `---` in `Yaml`. Over a `Seq<T>` the
  result is one whole text, `F`.
- `D2` and `Mermaid` describe a whole; formatting a `Stream` in them MUST be
  refused, naming `take` and `collect(n)`.
- A host MUST write a `Stream<F>` result piece by piece as the pieces arrive,
  and a stream result that is not formatted in its presentation context the
  same way. A reader that stops reading ends the stream. A JSON envelope is one
  document, so `--envelope json` MUST refuse a stream that may not end, naming
  `take`.

## Backwards Compatibility

Breaking. The `present` extension is removed and `format` takes its place in
the prelude. `table` becomes an unknown operation naming `Table`, and `graph`
becomes the binding of `vault.graph`; programs that pipe into the `graph`
projection change to `Graph`.
`json`, `text` and `markdown` keep their names and become shorthands, and their
results become readable text, so a step after them that used to be refused now
checks. `tree`, `value` and `empty` were never implemented and are dropped.
`--format` is renamed `--envelope`. A table is built as
`Table(xs, columns = [...])`.

## Reference Implementation

- `src/extensions/format.rs` — the `format` operation, the renderer table, the
  built-in renderers; replaces `src/extensions/present.rs`
- `std/format.hql` — the format types and the shorthands
- `src/constructors.rs` and `std/format.hql` — `Table` and its constructor
- `src/types/display.rs` — each format's encoder for a `Table`
- `src/extensions/graph.rs` — the `graph` step removed; `Graph` as a
  constructor; `expand` refusing depth `0`
- `src/extensions/mod.rs` — `format` in the prelude in place of `present`
- `src/extensions/mod.rs`, `Config` — the `format` configuration section
- `src/cli.rs` — `--to`, `--from`, `--envelope`
- `src/transclude.rs` — the `hmd` context for `hql render`
- `doc/wiki/hql/presentation.hmd` — rewritten around shape types, presentation
  formats, renderers and the context

## Test Plan

Unit tests MUST include:

- `card | markdown` is a `Markdown`
- `format(x, Markdown)` and `x | markdown` give the same text
- `cards | Graph | format(Json)` writes the node-link form with every card as a
  node
- `cards | Graph | markdown` with the default configuration writes a fenced
  `d2` block; with `renderer = "mermaid"` a Mermaid block
- `cards | Graph | format(Markdown, limit = 2)` renders two nodes and the cut
  marker
- `xs | d2 | markdown` fences the D2 text; `m: Markdown | hmd` copies nothing
- a type and format with no renderer is refused naming both
- `cards | graph` refused naming `Graph`; `cards | table` refused naming
  `Table`; `expand(depth = 0)` refused
- a configuration naming an unknown renderer is reported when read
- `Table(xs, columns = [...])` writes its columns in the order named in
  `Markdown`, `Csv`, `Json` and `Text`; a column naming no field of a record
  is refused; a table renderer over a value that is not a `Table` is refused

Integration tests MUST include:

- `hql run q.hql --to json` and `hql run q.hql` give node-link JSON and an edge
  list for the same graph query
- `hql render card.hmd` writes a graph cell as a fenced `d2` block and a
  `Markdown` result spliced in
- `--to none` writes no value and exits `0`; `--envelope json` carries the
  value and the reports

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-18: drafted
- 2026-10-18: accepted
- 2026-10-19: the format types, `format` and its shorthands, the renderers and
  their configuration, the presentation context, `--to`, `--envelope` and
  streams are implemented; `--from`, `Table`, `Tree` and parsing by
  construction (`s | Data`) are not yet
- 2026-10-22: parsing by construction is implemented as `Data(s, F)`, the
  format written, by HQL-0018
- 2026-10-22: `--from <format>` is implemented: it names the format of
  standard input where a program reads `stdin` and names none
- 2026-10-22: `Table` is implemented: `Table(xs, columns = [...])`, declared
  in `std/format.hql`, written by each format's encoder in column order; the
  table renderers write only a `Table`. "Shape types", "Renderers and their
  configuration" and "The presentation context" point to the
  [presentation](../../wiki/hql/presentation.hmd) card, which states the
  model
