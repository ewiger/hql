# HQL-0057: Packages: the `hql-extensions` registry, `hql package`, and installed extensions

**Status**: drafted
**Created**: 2026-11-13
**Source**: [HQL-0024 part 9](../HQL-0024/parts/09-packages-and-extensions.md),
whose transport, ABI and package-management records this is;
[HQL-0024 part 10](../HQL-0024/parts/10-host-boundary.md) (installed-extension
locations belong to the execution layer); [issue 0049](../../issues/0049-one-extension-per-package.md);
[issue 0077](../../issues/0077-packages-and-the-ml-extension.md)

## Companion notes

- [`parts/01-packages.md`](parts/01-packages.md) — the registry file
  `hql-packages.yaml`, the `hql-extensions` repository, the layout of an
  installed package, how a package is built and loaded, configuration, and
  `hql package`.
- [`parts/02-ml.md`](parts/02-ml.md) — the first official package, `ml`:
  datasets, fitted models as values, prediction and metrics.

## Abstract

A package is a Cargo crate that builds an HQL extension: a Rust library
exporting one descriptor and the implementations of the external
declarations of the modules it provides, beside those modules' contracts.
Packages come from four places. The two builtin packages, `std` and
`search`, are compiled into the binary. Official packages are the crates of
the [`hql-extensions`](https://github.com/ewiger/hql-extensions) repository,
a Cargo workspace whose root file `hql-packages.yaml` is the package
registry. Known packages are crates outside that repository which the same
file lists; a crate is added to the list by a pull request. Local packages
are built from a path on the machine. `hql package` installs, lists, updates,
rebuilds and removes packages into the HQL home, `~/.hql/lib/<package>/`,
building each with the user's `cargo` against the running `hql`. A program
reaches an installed package as it reaches a builtin one: `import ml` loads
the extension that provides `ml`, and nothing in HQL source names a package.
The first official package is `ml`.

## Motivation

- HQL-0024 part 9 fixed what a package, an extension and a module are, and
  deferred how an extension's code reaches the process and how a package is
  installed. Both are needed before anything outside the binary can
  implement a module, and every builtin module written so far has grown
  the binary instead.
- Machine learning does not belong in the binary: its dependencies are
  large, its users are few, and its surface will change faster than the
  language. It is the right first package precisely because it is the
  wrong first builtin.
- An extension is native code running with the process's rights. Who may
  provide one, and from where, has to be decided before the mechanism
  exists, not after. The registry is a list reviewed by pull request, and
  installation by name reaches only that list.
- Cargo already fetches, pins, verifies and builds Rust crates. A package
  manager of HQL's own would redo that work worse. `hql package` drives
  `cargo`; it does not replace it.

## Goals

- One mechanism from `hql package install <name>` to `import <module>` in
  a program, with the module behaving as a standard module does after
  binding.
- A package developed on the machine installs from its path and is used
  the same way.
- A build of a package that does not match the running `hql` is refused
  with a message that names the command that fixes it.

## Non-goals

- A stable ABI across `hql` versions. An installed package is built against
  the `hql` it runs in and is rebuilt after an upgrade.
- Dependency resolution between packages. A package depends on the `hql`
  crate and on Cargo crates; a package that needs another package's
  modules is outside this record.
- Loading out of process or as WASM. The descriptor's `abi` field is where
  such a transport attaches; this record pins `abi = 1`, the in-process
  library.
- Publishing packages to crates.io. The registry names where a package's
  source is; whether that source is crates.io or a git repository is the
  entry's business.

## Specification

The rules are in the companion notes. In short:

1. **A package is a crate.** Its name is its Cargo name with the prefix
   `hql-ext-` removed: `hql-ext-ml` is the package `ml`. It owns the
   top-level module namespace of that name, `ml` and `ml.*`, and may claim
   no other ([part 1, the package](parts/01-packages.md#1-a-package)).
2. **The registry is a file.** `hql-packages.yaml` at the root of
   `hql-extensions` lists every official and known package, each with its
   source and the `hql` versions it builds against
   ([part 1, the registry](parts/01-packages.md#2-the-registry)).
3. **A package installs into the HQL home.** `~/.hql/lib/<package>/` holds
   `package.toml`, the `modules/` root with the contracts, and the built
   library ([part 1, an installed package](parts/01-packages.md#3-an-installed-package)).
4. **A build is pinned to one `hql`.** `package.toml` records the `hql`
   version and the `rustc` the library was built with; a mismatch at load
   is refused, naming `hql package rebuild`
   ([part 1, building and loading](parts/01-packages.md#4-building-and-loading)).
5. **Discovery is configured; loading is by import.** The execution
   context discovers packages in the HQL home and in `[packages] paths`;
   an extension loads when a unit imports a module it provides, or when
   `[extensions] import` names it
   ([part 1, configuration](parts/01-packages.md#5-configuration)).
6. **`hql package` is the one command.** `install`, `list`, `info`,
   `search`, `update`, `rebuild`, `remove` and `registry`
   ([part 1, the command](parts/01-packages.md#6-hql-package)).
7. **`ml` is the first official package** ([part 2](parts/02-ml.md)).

## Backwards Compatibility

Nothing a program writes changes. `[extensions] import` and `prelude` keep
their meaning; `[packages]` is a new table. `hql builtins` names a package
beside the module it provides, which changes its output for the installed
case only. The `hql` crate gains a public module, `hql::ext`, for package
authors; the rest of the crate's public surface is unchanged.

## Security Considerations

- An extension is native code with the rights of the process. Installation
  by name reaches only the registry, whose every entry was reviewed in a
  pull request to `hql-extensions`; `--path` installs what the user built.
  There is no third way to install.
- A registry entry pins its source: a tag of `hql-extensions` for an
  official package, a crates.io version for a known one. Cargo verifies
  the checksums of what it fetches; `hql package` adds no fetching of its
  own.
- The registry file is fetched over HTTPS from the repository and cached;
  a cached copy is used when the fetch fails, and `hql package registry`
  shows which copy is in use and from when.
- A library in `~/.hql/lib/` is loaded only when its `package.toml`
  matches the running binary. A library placed there by hand without a
  matching record is reported by `hql doctor` and never loaded.

## Deployment / Activation

1. Fold the builtin descriptors into `std` and `search` (issue 0049) and
   give the descriptor its `id` and `abi`; expose `hql::ext`.
2. `hql package install --path`, `list`, `info`, `remove` and `rebuild`
   against a package on the machine, loading through `abi = 1`.
3. The `hql-extensions` repository with `hql-packages.yaml` and
   `hql-ext-ml`; `install <name>`, `search`, `update` and `registry`.
4. Known packages: the registry's `known` entries, and the pull-request
   rule in the repository's README.

## Reference Implementation

- `src/extensions/mod.rs`: `ExtensionDescriptor { id, abi, provides }` on
  the two internal extensions; a registry entry per installed package with
  an external origin.
- `src/ext.rs`, exported as `hql::ext`: the types a package implements
  against, `Extension`, `Operation`, `Value`, `EvalCx`, `TypedArg`, and
  the entry-point macro `hql::extension!`.
- `src/package/`: the registry file, `package.toml`, discovery, the
  `cargo` driver and loading with `libloading`.
- `src/cli.rs`: the `Package` command and its actions; `hql doctor`
  reports packages.
- `src/context.rs`: `[packages] paths`.
- `hql-extensions`: `Cargo.toml` workspace, `hql-packages.yaml`,
  `crates/hql-ext-ml/`.

## Test Plan

- Unit: the registry file parsed and refused where malformed; a
  `package.toml` matched and mismatched against the binary's version and
  toolchain; discovery with two packages providing one module refused.
- Integration, `tests/package.rs`: a fixture package under
  `tests/packages/hello/` built with `--path` into a temporary HQL home,
  then `import hello` in a program evaluates its one function; the same
  package with an edited `package.toml` refused at load with the message
  naming `hql package rebuild`; `hql package list` and `remove`.
- `hql-extensions` CI: builds every crate against `hql` `main` and runs
  each package's `examples/`.

## Changelog

- 2026-11-13: drafted
