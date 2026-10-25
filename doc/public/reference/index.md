# Reference

## Commands at a glance

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql help <name>` | the declarations of a function, type, trait or module, with their doc strings |
| `hql builtins` | list the steps |
| `hql config` | the reporting mode and shown level in force, and where each came from |
| `hql doctor` | check the semantic-search chain and name the first missing part |
| `hql ai` | list the model requests waiting under `--ai host`, and answer one (next release) |

`hql '<program>'` is `hql eval '<program>'`. `-` reads standard
input wherever a file is taken. Success is exit `0`, a
language or I/O failure `1`, a usage error `2`.

## Doc strings

A block of `///` lines opened after an empty line is the doc string of the
declarations below it, up to the next empty line. `hql help <name>` and
`:help <name>` in `hql repl` print each declaration of a name with its module
and doc string; `:type <name>` prints the first sentence. The pages below are
the standard library's doc strings, one page per function, type and trait.

<!-- hql reference: index -->
## The standard library

- [`std/core`](std/core/index.md) — The types every other module is written in: the scalars, the collection types, the open tree `Data`, absence, failure, value order, the format category and the core traits.
- [`std/doc`](std/doc/index.md) — Documents: a header of everything known about one and a body someone wrote, and the card, a document a vault holds.
- [`std/retrieval`](std/retrieval/index.md) — Retrieval: a corpus ranked against a query.
- [`std/graph`](std/graph/index.md) — Graphs: nodes and the edges among them, structural only; what a node or an edge means is the `knowledge` module's.
- [`std/knowledge`](std/knowledge/index.md) — Knowledge: the semantic domain, where everything is a claim.
- [`std/collections`](std/collections/index.md) — The operations over the collection types `core` declares: membership, filtering, mapping, order, grouping, joins and reductions.
- [`std/string`](std/string/index.md) — The functions of a `String`, in the prelude.
- [`std/seq`](std/seq/index.md) — The sequence, set and map operations beyond the core's membership, order and lookup, in the prelude.
- [`std/math`](std/math/index.md) — Numbers, in the prelude.
- [`std/option`](std/option/index.md) — The ways out of an `Option<T>`, in the prelude, each stating what absence gives.
- [`std/result`](std/result/index.md) — The ways out of a `Result<T, E>`, in the prelude, each stating what a failure gives.
- [`std/regex`](std/regex/index.md) — Text matched against a regular expression.
- [`std/format`](std/format/index.md) — The formats, how bytes are read as values and how values are written as text, and the codec contracts over them.
- [`std/io`](std/io/index.md) — Input and output: endpoints, the data sources read from them, and the data targets written to them.
- [`std/io/net`](std/io/net/index.md) — Network endpoints: transports, read and written by `source`, `tap`, `target` and `sink` like any other endpoint (module `io`).
- [`std/io/logging`](std/io/logging/index.md) — Logging: structured events written to a destination, apart from the result.
- [`std/vault`](std/vault/index.md) — The current vault: the source a program runs against.
- [`search/lexical`](search/lexical/index.md) — Lexical retrieval: a corpus ranked by the words it shares with a query, a hashed bag of words compared by cosine.
- [`search/semantic`](search/semantic/index.md) — Semantic retrieval: a corpus ranked against a query by vectors a language model made ahead of time, read from the index that `[semantics] index` in hql.toml names.
- [`std/validators`](std/validators/index.md) — Validators: the standard validator types, imported where they are used, `import validators`, and never by the prelude, so a program's own `Email` is its own.
<!-- hql reference: end -->
