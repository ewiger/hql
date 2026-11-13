# File system: folders, globs and entries

The cases follow [HQL-0040](../../doc/proposals/HQL-0040/README.md). `io`
declares `Path`, a place in a file system written as text, and its pure
operations; the `io.fs` module, imported with `import io.fs`, declares
`Folder`, `Glob` and `Entry`, and the effectful operations that list a
folder, walk a tree, match a glob and test a path. `tree/` is the folder
they list:

```text
tree/
  assets/logo.txt        5 bytes
  assets/style.css      19 bytes
  data/2026/q1.csv      21 bytes
  data/2026-notes.md    17 bytes
  data/birds.csv        28 bytes
```

| Case | What it shows | Result |
| --- | --- | --- |
| [walk order](queries/walk-order.hql) | `walk` in sorted-walk order: `2026/q1.csv` before `2026-notes.md` | four paths |
| [largest CSV](queries/largest-csv.hql) | `glob(Glob("**/*.csv"))` sorted by `size` | `["birds.csv", "q1.csv"]` |
| [folders](queries/folders.hql) | `entries` of one folder, kept by `kind` | `["assets", "data"]` |
| [bytes](queries/bytes.hql) | the sizes of the files under `assets/` summed | `24` |
| [rows](queries/rows.hql) | an entry's `path` read with `file` | the species of `birds.csv` |
| [path operations](queries/path-operations.hql) | `join`, `name`, `stem`, `extension` and `parent` | six texts |
| [exists](queries/exists.hql) | `exists` and `entry` over a missing path | `false` and `none` |
| [invalid glob](queries/invalid-glob.hql) | `Glob("data/a**.csv")` | an evaluation error |
| [missing folder](queries/invalid-missing-folder.hql) | `entries` of a folder that is not there | an evaluation error naming it |
| [not imported](queries/invalid-not-imported.hql) | `folder` without `import io.fs` | a name error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/fs/queries/walk-order.hql
```

A relative path resolves against the program that names it, so each case
lists `../tree` beside `queries/`.
