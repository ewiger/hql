# HQL

**Hyper Query Language**

A typed functional language for querying and computing over structured knowledge.

HQL is being developed alongside [HyperMarkDown](https://github.com/ewiger/hypermarkdown)
and is intended to operate over its cards, document structure, metadata, and
knowledge graph. HyperMarkDown remains the authored/storage representation;
HQL is a separate language and repository.

**Experimental bootstrap:** currently only integer, float and Boolean literals
and same-type numeric addition work. There is no HyperMarkDown integration yet.

## Develop and run

Install Rust through rustup, then run from this repository:

```sh
cargo build --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked -- eval '40 + 2'           # 42
cargo run --locked -- eval '0.5 + 0.25'       # 0.75
cargo run --locked -- eval 'true'             # true
cargo run --locked -- check examples/answer.hql  # Int
```

The generated toolchain selects `stable` with rustfmt and clippy; the manifest
requires Rust 1.85 or newer (edition 2024). Cargo.lock records dependencies.

To install the `hql` command locally:

```sh
cargo install --path . --locked
hql eval '40 + 2'
hql check examples/answer.hql
```

`check` parses and checks a UTF-8 file containing one expression; it prints its
type without evaluating it. `eval` prints a value. Language and I/O failures go
to stderr with exit status 1; CLI usage errors use status 2. Diagnostic ranges
are zero-based UTF-8 byte offsets. File extensions are not enforced.

## Example corpus

[108 design cases](examples/README.md) distinguish current behavior, proposed
syntax, intentional errors, and open alternatives. Run the supported corpus
checks with `cargo test --locked --test corpus`.

## Project knowledge

- [System requirements](doc/models/requirements/bootstrap.md)
- [Architecture](doc/models/domain/architecture.md)
- [Current expression contract](doc/models/behavior/expressions.md)
- [HQL overview](doc/wiki/hql.hmd)
- [HyperMarkDown integration](doc/wiki/hmd-integration.hmd)
- [Open design questions](doc/wiki/design-direction.hmd)
- [Bootstrap decisions and provenance](doc/memory/bootstrap.md)

Scaffolded with [grem](https://github.com/ewiger/grem)'s Rust template using
`grem init . -t rust --name hql` in an empty `hql` directory.

## License

MIT — see [LICENSE](LICENSE).
