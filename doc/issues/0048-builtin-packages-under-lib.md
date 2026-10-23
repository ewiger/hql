# 0048 — Builtin packages live under `lib/`: `std` is one of them

Status: doing
Related: [`lib/std/`](../../lib/std/README.md),
[HQL-0024 part 3](../proposals/HQL-0024/parts/03-standard-library.md),
[packages](../status/packages.hmd),
[issue 0046](0046-renovate-std.md)

The binary carries builtin packages, and `std` is one of them. Each builtin
package is a folder under `lib/`: its contracts are bundled with the binary
and its extensions compiled into it. `std` is the standard package, the one
whose `prelude.hql` is the implicit import list. Other builtin packages ship
with their own extensions and are named for what they provide, not `std`.

## Work

1. Move `std/` to `lib/std/`, history kept (`git mv`).
2. The binary embeds `lib/std/`: the `include_str!` paths of
   `src/library.rs`, the written module's located path in `src/checker.rs`,
   and the tests that walk or check the folder (`tests/declarations.rs`,
   `tests/cli.rs`).
3. A full reference pass: every ADR under `doc/proposals/`, every issue,
   card, model and status document, the README, the examples, and the
   comments in `src/`, `tests/` and `lib/std/` name `lib/std/…` where they
   named the folder `std/…`, and every relative link reaches `lib/std/`.
   Diagnostics keep naming a module as `std/core`: that is the module's
   package and name, not a path. Released `CHANGELOG.md` entries keep the
   paths they shipped with.
4. The package model states builtin packages in the plural: `lib/` holds
   one folder per builtin package, `std` the standard one. HQL-0024 part 3,
   the std-lib card, `lib/std/README.md` and `doc/status/packages.hmd` say
   so.
5. A second builtin package, `search`: its internal extension `search`
   provides `lexical` and `semantic`, and no contract declares them yet.
   Give them contracts under `lib/search/`. Folding the descriptors is
   [issue 0049](0049-one-extension-per-package.md).
