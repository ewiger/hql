# 0077 — Packages: `hql-extensions`, `hql package`, and the `ml` extension

Status: doing
Related: [HQL-0057](../proposals/HQL-0057/README.md),
[HQL-0024 part 9](../proposals/HQL-0024/parts/09-packages-and-extensions.md),
[0049](0049-one-extension-per-package.md)

HQL-0024 part 9 named the package, the extension and the module and left
how a package is installed and how its code reaches the process to a later
record. HQL-0057 is that record: a package is a Cargo crate, the registry is
`hql-packages.yaml` in the `hql-extensions` repository, `hql package`
builds and installs into `~/.hql/lib/`, and `ml` is the first official
package. This issue carries the record and the first package into code,
on `feat/extensions-ml`.

## Work

1. HQL-0057 drafted, with part 1 (packages) and part 2 (`ml`).
2. The `hql-extensions` repository: workspace, `hql-packages.yaml`,
   `crates/hql-ext-ml` with its contracts, README and CI.
3. Issue 0049 as the prerequisite: two descriptors with `id`, `abi` and
   `provides`.
4. `hql::ext`, the entry point, and a fixture package under
   `tests/packages/hello/`.
5. Discovery, `package.toml`, loading and the version check;
   `hql package install --path`, `list`, `info`, `remove`, `rebuild`.
6. The registry fetch and cache; `install <name>`, `search`, `update`,
   `registry`.
7. `ml`'s natives over `linfa`, and its examples.
8. Chapter 13 on `[packages]` and `hql package`; the book's Part IV chapter
   on declarations and extensions.

## Done when

- `hql package install ml` on a machine with `cargo`, then `import ml` in
  a program, fits and predicts as part 2's program does.
- The fixture package proves the mechanism in `cargo test` without the
  network.
