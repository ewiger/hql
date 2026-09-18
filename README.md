# HQL

**Hyper Query Language**

A typed functional language for querying and computing over structured knowledge.

HQL is being developed alongside [HyperMarkDown](https://github.com/ewiger/hypermarkdown)
and is intended to operate over its cards, document structure, metadata, and
knowledge graph. HyperMarkDown remains the authored/storage representation;
HQL is a separate language and repository.

**Experimental.** The language is in design. What runs today is a typed
expression core, a vault of Markdown and HyperMarkDown documents, pipelines over
its cards, semantic retrieval and graph traversal. Most of `doc/models/` is still
design, and the implementation says so rather than faking it.

## Develop and run

Install Rust through rustup, then run from this repository:

```sh
cargo build --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

```sh
cargo run -- eval '40 + 2'                       # 42
cargo run -- --vault <dir> eval 'cards | count'
cargo run -- builtins                            # the steps a pipeline may use
```

To install the `hql` command locally:

```sh
cargo install --path . --locked
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
| graph
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

Two retrievals, each named for what it does. `lexical` matches shared spellings,
offline and with nothing to build. `semantic` scores against vectors a language
model computed ahead of time, which
[`contrib/semantics/`](contrib/semantics/README.md) produces and the binary only
reads — the model is not in here and will not be.

Either way the answer keeps its evidence. Every hit carries the retrieval that
produced it — query, index, model, model revision, metric, and whether the
search was approximate — because a score is evidence rather than relevance.
`take` needs elements that carry an order of their own, which a card does, so a
prefix is reproducible without the vault's file order ever being observable.
`expand` traverses outwards and records *why* each node is present, so a graph
of three matches and their neighbours does not claim that all of them matched.

### Commands

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL blocks in a document, transcluding the answers |
| `hql builtins` | list the steps |
| `hql config` | the reporting mode in force, and where it came from |

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
| table
```
````

`hql render --write board.md` writes the answer beneath it as an `hql#result`
block, replacing the previous one, so a to-do list is a query rather than a list
somebody maintains.

### Reporting

A run yields a value *and* a queue of everything it had to say: warnings that
change nothing, errors that make a result wrong, failures that make one
impossible. `--report strict` stops at the first error and is the default;
`--report collect` reports every error findable. The mode comes from the query,
then the vault's `hql.toml`, then `$HQL_REPORT`, then the default.

Success is exit `0`, a language or I/O failure `1`, a usage error `2`.
Diagnostics carry zero-based UTF-8 byte ranges and are rendered with a line, a
column and a caret.

## Example corpus

[Design cases](examples/README.md) distinguish current behavior, proposed syntax,
intentional errors, and open alternatives. Run the supported corpus checks with
`cargo test --locked --test corpus`; they skip when the corpus is not present in
the working tree.

## Project knowledge

- [System requirements](doc/models/requirements/bootstrap.md)
- [Architecture](doc/models/domain/architecture.md)
- [The implemented grammar](doc/models/behavior/expressions.md)
- [The command-line host](doc/models/behavior/cli.md) and [reporting](doc/models/behavior/reporting.md)
- [The knowledge](doc/models/domain/knowledge.md), [graph](doc/models/domain/graphs.md) and [semantic search](doc/models/domain/semantic-search.md) domains
- [HQL overview](doc/wiki/hql.hmd)
- [HyperMarkDown integration](doc/wiki/hmd-integration.hmd)
- [Open design questions](doc/wiki/design-direction.hmd)
- [Bootstrap requirements](doc/models/requirements/bootstrap.md)

Scaffolded with [grem](https://github.com/ewiger/grem)'s Rust template using
`grem init . -t rust --name hql` in an empty `hql` directory.

## License

MIT — see [LICENSE](LICENSE).
