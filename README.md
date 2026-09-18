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

```sh
hql --vault notes/ eval '
cards
| semantic("bearer token authorization")
| take(5)
| expand(depth = 1)
| graph
| table'
```

`cards` is the vault's cards. `semantic` ranks them against a query and keeps
the retrieval — model, metric, index — with every score, because a score is
evidence rather than relevance. `take` needs elements that carry an order of
their own, which a card does, so a prefix is reproducible without the vault's
file order ever being observable. `expand` traverses outwards and records *why*
each node is present, so a graph of five matches and their neighbours does not
claim that all of them matched.

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
