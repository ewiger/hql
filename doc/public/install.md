# Install

## From a release

Each release ships a binary for every supported target, with a checksum beside
it. Download the pair from the
[releases page](https://github.com/ewiger/hql/releases), check it, and put
`hql` somewhere on your `PATH`:

```sh
shasum -a 256 -c hql-<version>-<target>.tar.gz.sha256   # sha256sum -c on Linux
tar -xzf hql-<version>-<target>.tar.gz
```

The targets are `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` and
`x86_64-apple-darwin`. An archive that does not match its checksum is not the
one that was built.

## With Cargo

With Rust 1.88 or newer from [rustup](https://rustup.rs):

```sh
cargo install hql --locked                                   # the last release
cargo install --git https://github.com/ewiger/hql --locked   # the latest commit
cargo install --path . --locked                              # from a clone
```

## Check it works

```sh
hql eval '40 + 2'
```

```text
42
```

`hql builtins` lists every step a pipeline may use, grouped by the extension
that provides it.

!!! note "Versions"

    While the major version is `0`, a minor release may break a program that
    ran before. The
    [changelog](https://github.com/ewiger/hql/blob/main/CHANGELOG.md) says so
    release by release.
