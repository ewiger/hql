# HQL

[![CI](https://github.com/ewiger/hql/actions/workflows/ci.yml/badge.svg)](https://github.com/ewiger/hql/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Hyper Query Language** — documentation at
[hql.hypermarkdown.org](https://hql.hypermarkdown.org/)

HQL is a statically typed expression and query language. A program is
type-checked before it is evaluated. Values are transformed through pipelines
of operations such as `filter`, `map`, `sort`, retrieval and graph traversal,
over a vault of Markdown, HyperMarkDown, YAML and JSON documents.

HyperMarkDown is the document format most HQL vaults are written in. HQL does
not define or change that format; it reads
[HyperMarkDown](https://github.com/ewiger/hypermarkdown), plain Markdown, YAML
and JSON, and is a separate language in its own repository.

**In development.** The binary implements a statically typed expression core,
a vault of Markdown and HyperMarkDown documents, pipelines over its cards,
`lexical` and `semantic` retrieval, and graph traversal. `doc/models/` contains
both implemented semantics and design proposals; each document states its
implementation status.

Retrieval, graph traversal and presentation are extensions: providers of typed
steps registered in the binary. The implemented extensions are `core`,
`graph`, `present`, `lexical` and `semantic`. The design direction is
extensions over further sources, so that one program queries across SQL
databases such as Postgres, JSON and YAML documents, message queues such as
Kafka, and REST APIs. HQL's operations take their inspiration from jq and SQL,
among others; [the jq parity analysis](doc/issues/0015-jq-parity-analysis.md)
and [the SQL parity analysis](doc/issues/0016-sql-parity-analysis.md) record
which operations are adopted.

## Install

A release ships a binary for each supported target, with a checksum beside it.
Download the pair from the [releases page](https://github.com/ewiger/hql/releases),
check it, and put `hql` somewhere on your `PATH`:

```sh
shasum -a 256 -c hql-<version>-<target>.tar.gz.sha256   # sha256sum -c on Linux
tar -xzf hql-<version>-<target>.tar.gz
```

The targets are `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` and
`x86_64-apple-darwin`. Verify the archive against the published SHA-256
checksum before extracting it.

To build it yourself instead, with Rust 1.88 or newer from
[rustup](https://rustup.rs) — the floor is `rust-version` in `Cargo.toml`, and CI
builds on it so the declared floor stays true:

```sh
cargo install hql --locked                                   # the last release
cargo install --git https://github.com/ewiger/hql --locked   # the latest commit
cargo install --path . --locked                              # from a clone
```

[CHANGELOG.md](CHANGELOG.md) records what changed in each release. While the
major version is `0`, a minor release may change the language incompatibly,
and the changelog lists each such change by release.

## Develop and run

[DEVELOP.md](doc/DEVELOP.md) covers building, testing and linting. It also shows
how to put the development build on your `PATH` as `hql`, linked to your clone
so a rebuild updates it:

```sh
cargo build --release
ln -sf "$PWD/target/release/hql" ~/.cargo/bin/hql   # or ~/.local/bin/hql
```

### Querying a vault

`tests/fixtures/birds/` is a worked example: fifty-six encyclopedia articles
about birds, with an embedding index committed beside them.

```sh
hql --vault tests/fixtures/birds eval '
import semantic

cards
| semantic("night hunting birds")
| take(3)
| expand(depth = 1)
| table'
```

That returns the owls. Not one of their articles contains the word "night" —
they are nocturnal, and abroad after dark — so the same query through the other
retrieval finds nothing of the kind:

```sh
hql --vault tests/fixtures/birds eval '
import lexical

cards
| lexical("night hunting birds")
| take(3)
| map(h => h.card.name)'
```

```text
[mute-swan, passeriformes, alcedinidae]
```

`lexical(query)` ranks cards by textual match with the query and needs no
index. `semantic(query)` ranks cards by embedding similarity against an index
that [`contrib/semantics/`](contrib/semantics/README.md) builds ahead of time;
the binary reads the index and never loads the model.

Each hit carries its retrieval provenance: query, index, model, model revision,
metric, and whether the search was approximate. `take` requires elements with
an ordering key of their own, which a card has, so a prefix is reproducible
independently of the vault's file order. `expand` returns a `Graph` whose
nodes each store a presence: `retrieved` with a score, or `expanded from
<seed> at depth <n>`, so a node that matched is distinguishable from a node
that was reached.

### Commands

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql builtins` | list the steps |
| `hql config` | the reporting mode and shown level in force, and where each came from |
| `hql doctor` | check the semantic-search chain — vault, `hql.toml`, index, model, query vectors, card coverage — and name the first missing part |

`-` reads standard input wherever a file is taken. `--format json` emits the
type, the value and the report queue for another program.

### Queries inside documents

A card can carry a query, and rendering it runs the query against the vault:

````markdown
```hql#eval
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```
````

`hql render board.hmd --write board.md` replaces each cell with its output as
plain Markdown, here a bulleted list, so a to-do list is a query rather than a
list somebody maintains. `--emit after` keeps the cell and writes the output
after it, between `<!-- hql:output -->` and `<!-- /hql:output -->`; with
`--in-place` that rewrites the source, and the next render replaces the output
between the markers.

### Reporting

A run yields a value *and* a queue of everything it had to say: warnings that
change nothing, errors that make a result wrong, failures that make one
impossible. `--report strict` stops at the first error and is the default;
`--report collect` reports every error findable. The mode comes from the query,
then the vault's `hql.toml`, then `$HQL_REPORT`, then the default.
`xs | debug(label = "xs")` captures a value into the same queue and passes it
on unchanged; `--report-level debug` shows it, and `--format json` always
carries it.

Success is exit `0`, a language or I/O failure `1`, a usage error `2`.
Diagnostics carry zero-based UTF-8 byte ranges and are rendered with a line, a
column and a caret.

## Example corpus

[The birds wiki](examples/birds/README.md) pairs linked HyperMarkDown cards with
runnable collection queries: repeated sightings, distinct species, keyed counts,
ordered routes, and sorted keys. It also includes intentional errors for
duplicate keys and non-orderable sorted-map keys.

## Project knowledge

- [System requirements](doc/models/requirements/bootstrap.md)
- [Architecture](doc/models/domain/architecture.md)
- [The implemented grammar](doc/models/behavior/expressions.md)
- [The command-line host](doc/models/behavior/cli.md) and [reporting](doc/models/behavior/reporting.md)
- [The knowledge](doc/models/domain/knowledge.md), [graph](doc/models/domain/graphs.md) and [semantic search](doc/models/requirements/semantic-search.md) domains
- [HQL overview](doc/wiki/hql.hmd)
- [HyperMarkDown integration](doc/wiki/hmd-integration.hmd)
- [Open design questions](doc/wiki/design-direction.hmd)
- [Bootstrap requirements](doc/models/requirements/bootstrap.md)

Scaffolded with [grem](https://github.com/ewiger/grem)'s Rust template using
`grem init . -t rust --name hql` in an empty `hql` directory.

## License

MIT — see [LICENSE](LICENSE).
