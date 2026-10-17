# Develop

How to build, check and run HQL from a clone. [The stack](stack.md) says how the
code is written; this page says how to work on it.

## Prerequisites

Rust from [rustup](https://rustup.rs). `rust-toolchain.toml` pins the channel and
the `rustfmt` and `clippy` components, so rustup installs the right toolchain on
the first `cargo` command in the repository.

## Build and check

```sh
cargo build --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

CI runs the same four commands. A clippy warning fails the build.

## Run from the source tree

```sh
cargo run -- eval '40 + 2'                       # 42
cargo run -- --vault <dir> eval 'cards | count'
cargo run -- builtins                            # the steps a pipeline may use
```

## Put the development build on your `PATH`

To call `hql` from any directory while it still runs your checkout, link the
release build into a directory on your `PATH` instead of installing a copy:

```sh
cargo build --release
ln -sf "$PWD/target/release/hql" ~/.cargo/bin/hql   # or ~/.local/bin/hql
```

The link points at `target/release/hql`, so every `cargo build --release` updates
the global `hql` and nothing has to be reinstalled. A Rust binary still has to be
compiled: an edit takes effect after the next release build, not before.

`cargo install --path .` is different. It copies the binary into
`~/.cargo/bin`, and that copy does not change when you rebuild.

`cargo clean` deletes the link's target, and `hql` fails until the next release
build. To go back to an installed release, remove the link first
(`rm ~/.cargo/bin/hql`). Otherwise `cargo install` refuses to overwrite a binary
it did not install.
