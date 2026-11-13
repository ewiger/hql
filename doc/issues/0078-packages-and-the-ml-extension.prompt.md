# Prompt — issue 0078

Paste the block below into a new session opened in the worktree
`~/code/ai/hmd/hql-feat-extensions-ml` (branch `feat/extensions-ml`), or run
it with `/loop`. Each run does one item of the issue's "Work" list and ends
with a commit.

---

Carry out [issue 0078](0078-packages-and-the-ml-extension.hmd): bring
[HQL-0057](../proposals/HQL-0057/index.hmd) into code, so that `hql package
install` builds a package with the user's `cargo` into `~/.hql/lib/`, a
program's `import` loads it, and the first official package, `ml`, fits and
predicts.

Read `doc/memory/` first, as `CLAUDE.md` requires, then `doc/stack.md`, the
issue, HQL-0057 and both its parts, and HQL-0024 part 9 as you reach it.
Ignore `.grem/**`.

## Where things are

- This repository's worktree is `~/code/ai/hmd/hql-feat-extensions-ml` on
  `feat/extensions-ml`, a sibling of `~/code/ai/hmd/hql` (`main`). Work
  here only; never `cd` to the main checkout.
- The package repository is `~/code/ai/hmd/hql-extensions`, a sibling: a
  Cargo workspace, `hql-packages.yaml` as the registry, and
  `crates/hql-ext-ml/` with its contracts under `modules/`. It is
  scaffolded and builds, but has no git history yet; its first commit is
  item 2 below. It is not on GitHub; ask before creating
  `ewiger/hql-extensions` or pushing anything.
- The `ml` contracts exist twice, in `crates/hql-ext-ml/modules/` and in
  HQL-0057 part 2. Keep them in step; `hql check <file>` on a contract must
  print `Step`.

## State on 2026-11-13

Done: HQL-0057 drafted (README, part 1 packages, part 2 `ml`); issue 0078
on the board in `doing`; the `hql-extensions` scaffold committed as the
first commit on its `main` (not on GitHub); issue 0049 closed in the
archive: `src/extensions/mod.rs` registers `STD` and `SEARCH`, each an
`Extension { id, abi, origin, provides }` whose `provides` is a list of
`Provided { module, steps }`, so an implementation is keyed by module and
declared name (`Extension::implementation`); `Resolution` binds modules,
not extensions, and `[extensions] import` takes a package id too. Next is
item 4.

## Work, in order

1. Already done: HQL-0057 drafted.
2. Already done: the `hql-extensions` scaffold committed.
3. Already done: issue 0049.
4. `hql::ext`: a public module exporting what a crate implements against
   (`Extension`, `Operation`, `Value`, `EvalCx`, `TypedArg`, the helpers
   `declared`/`effectful`) and the macro `hql::extension!` that writes the
   `hql_extension_v1` entry point. A fixture package under
   `tests/packages/hello/` with one module `hello` and one function,
   built in `tests/package.rs`.
5. Discovery and loading: `package.toml` read and written
   (`src/package/`), discovery from `~/.hql/lib/` and `[packages] paths`
   (`src/context.rs`), loading with `libloading`, the version and
   toolchain check with the exact refusal text of part 1 section 4, and
   two packages providing one module refused. `hql package install
   --path`, `list`, `info`, `remove`, `rebuild` in `src/cli.rs`.
   `hql doctor` reports packages. Two things part 3 (prebuilt libraries,
   a later stage, not implemented in this issue) asks of this item: the
   load check compares an exact `rustc`, so pin the exact version in
   `rust-toolchain.toml` in place of `channel = "stable"` while writing
   the check; and keep the source build of `install` in one function that
   fetches, builds and installs, so a prebuilt-first path can later sit in
   front of it without restructuring.
6. The registry: fetch and cache of `hql-packages.yaml`, `install <name>`,
   `search`, `update`, `registry [--refresh]`. The fetch must be
   mockable so `cargo test` stays offline.
7. `ml`: natives over `linfa-linear`, `linfa-logistic` and
   `linfa-clustering` in `crates/hql-ext-ml/src/`, the crate depending on
   `hql` by path during development; promote `examples/linear.hql` and add
   one example per model, the metrics, and each refusal part 2 names.
8. Docs: chapter 13 gains `[packages]` and `hql package`; a chapter on
   declarations and extensions at the end of the book's Part IV
   (title "32. Declarations and extensions: how the standard library is
   built"), written for a user who reads `lib/std/` or writes a package,
   with "Design only" admonitions on what is not shipped; `CHANGELOG.md`
   under Unreleased; HQL-0057's changelog and status as items land.

## How to work

- Proposed examples first: write the example with `// status: proposed`,
  make it pass, promote it to `valid-now`. Re-read the HQL-0057 diff
  mid-session; where code and record disagree, fix one and say which.
- Memory rules apply: snake_case names; docs state semantics, not
  intuition; no "Open Questions" sections; no decision archaeology in
  cards; no attribution of tools or assistants anywhere, including commit
  trailers.
- One commit per item, topic-wise, on `feat/extensions-ml`; never push
  without being asked. `cargo test` and `cargo clippy` clean before each
  commit; `cargo status` refreshes `doc/status/` when the board or a
  proposal's status changes.
- Finish by updating the issue's "Work" list and this prompt's "State"
  section so the next run starts where this one ended.
