# Stack

hql is built on this stack. Read it before adding a dependency,
picking a tool, or writing code — it is the contract contributors and agents
follow, so keep it current when the stack changes.

**Scope: the implementation, not the language.** Everything here governs the
Rust program that parses and evaluates HQL. None of it governs HQL's own design.
The host language's defaults, idioms and vocabulary carry no authority over what
HQL's syntax, type system or semantics should be — an argument for an HQL feature
is never "Rust does it this way". HQL is a data language and diverges freely
where that serves it; `doc/models/` and `doc/wiki/` are where its design is
argued. See [extensions](wiki/hql/extensions.hmd), which draws the same boundary
for what extensions may expose.

## Implementation language

- Rust, edition 2024. `rust-version` in `Cargo.toml` is the floor, and
  `rust-toolchain.toml` pins the channel everyone builds with.
- No `unsafe`. `src/lib.rs` carries `#![forbid(unsafe_code)]`; lifting it is a
  decision, not a detail.
- Fallible work returns `Result`. Panics are for invariants that a caller
  cannot violate, never for expected failures such as bad input or missing
  files.
- Prefer the standard library. Add a crate only when it removes significant,
  well-understood work.

## Types

- Make illegal states unrepresentable: newtypes instead of bare `String` and
  `u64`, `enum` for closed sets of values, and exhaustive `match` over a
  catch-all arm.
- `Option` and `Result` are part of the interface. No `unwrap()` or `expect()`
  outside tests and `main`; propagate with `?` and add context at the boundary.
- Borrow in parameters (`&str`, `&[T]`) and own in return values. Reach for
  `Cow`, `Arc`, or an explicit lifetime only when the borrow is measured.
- A clippy warning is a defect, not a warning.

## Toolchain and packaging

- [rustup](https://rustup.rs) with the pinned `rust-toolchain.toml` is the only
  supported workflow: `cargo build`, `cargo run`, `cargo test`,
  `cargo add <crate>`. Do not install crates by hand or edit `Cargo.toml`
  dependency entries that `cargo add` maintains.
- `Cargo.toml` is the single manifest. Development-only dependencies belong in
  `[dev-dependencies]`.
- `Cargo.lock` is committed and is the reproducible resolution. Regenerate it
  with `cargo add` or `cargo update`; never edit it by hand.
- Library core in `src/lib.rs`, a thin binary in `src/main.rs`, integration
  tests in `tests/`. Logic that only the binary can reach is logic that cannot
  be tested.

## `contrib/`

`contrib/` holds tooling that is not part of the binary and that `cargo test`
never invokes. It may be written in another language, and
[`contrib/semantics/`](../contrib/semantics/README.md) is: the embedding model
that produces an HQL index belongs where the model ecosystem is, and the
consumer belongs in Rust. Anything under `contrib/` MUST be runnable on its own,
MUST carry its own tests and its own README, and MUST NOT be a build dependency
of the crate — a contributor with no Python installed builds and tests
everything.

## Tests

- `cargo test`. Unit tests live in a `#[cfg(test)] mod tests` beside the code
  they cover; integration tests live in `tests/` and use the crate's public
  API only.
- Tests are deterministic and offline: no network, no wall-clock or
  machine-specific values, `tempfile` for anything touching the filesystem.
- A behavior change lands with the test that pins it.

## Code conventions

- Every module opens with a `//!` doc comment saying what it is for. Public
  items carry a one-sentence `///` doc; comments explain why, not what.
- Run `cargo fmt` and `cargo clippy -- -D warnings` before every commit.
  Formatting is rustfmt's job, not a review topic.
- Keep functions small and pure, and push I/O to the edges so the core stays
  testable.
- Define a crate-specific error `enum` with `thiserror` and return it from
  library code. `anyhow` is for the binary, where errors become messages.
- Match the surrounding code: naming, import order, and comment density are
  local conventions, not global ones.

## Default choices

Reach for these before introducing an alternative:

| Need | Choice |
| --- | --- |
| Toolchain and packaging | cargo + rustup |
| Edition | 2024 |
| Tests | `cargo test` (`insta` for snapshots) |
| CLI | clap (`derive`) |
| Serialization | serde |
| YAML headers and reference annotations | serde_yaml_ng, then fallible conversion to Data |
| SQLite | rusqlite (`bundled`) |
| Hashing | sha2 |
| Config files | toml |
| Errors | thiserror (library), anyhow (binary) |
| Logging | tracing |
| Version numbers | SemVer 2 (`semver`) |

Changing the stack is a decision, not a detail. Record a small one in
`doc/memory/` and a substantial one as a numbered proposal under
`doc/proposals/`, then update this file.

YAML parsing belongs to [`serde_yaml_ng`](https://docs.rs/serde_yaml_ng/latest/serde_yaml_ng/);
HQL owns only the conversion to `Data`.
Headers and annotations must be mappings. Non-string keys, custom tags,
non-finite numbers, and integers outside `i64` are loader errors, not text.
Invalid documents are skipped with a path-qualified warning. This replaces the
former indentation and comma-splitting parser as part of issue 0011.
