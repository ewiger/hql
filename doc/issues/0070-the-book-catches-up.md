# 0070 — The HQL book catches up with the general-purpose leap

Status: todo
Related: [0054](0054-user-facing-website.md),
[0067](0067-std-crypto.md),
[HQL-0025](../proposals/HQL-0025/README.md),
[HQL-0039](../proposals/HQL-0039/README.md),
[HQL-0040](../proposals/HQL-0040/README.md),
[HQL-0041](../proposals/HQL-0041/README.md),
[HQL-0053](../proposals/HQL-0053/README.md)

Branches `feat/gen-purpose-leap` and `feat/stats-env` implement a run of
standard modules and language features that the reference pages document
and the book does not. A reader of [the HQL book](../public/learn/index.md)
learns none of `Bytes`, files and folders, random values, statistics,
digests, encodings or the environment. The book is the one place that
teaches them in order. This issue brings it level with the code, keeping
its three parts, and adds a crypto demo to `hql-demo`.

## What shipped, and where the book stands

| Feature | Record | Branch | Book today |
| --- | --- | --- | --- |
| Traits: validator traits, `trait A as F`, module trait lists | HQL-0039 | gen-purpose-leap | [Traits](../public/learn/10-the-type-system/traits.md), current |
| Execution contexts, named contexts, `HQL_PATH` | HQL-0025, in part | gen-purpose-leap | [Working with hql](../public/learn/13-working-with-hql.md), to be checked |
| `io.fs`: `Path`, `Folder`, `Glob`, `Entry` | HQL-0040 | gen-purpose-leap | none |
| `random`: sampling, shuffling, numbers, UUIDs | HQL-0041 part 4 | gen-purpose-leap | none |
| `crypto.digest`: digests and fingerprints | HQL-0041 part 3 | gen-purpose-leap | none |
| `encoding`: hex, base64 and the other text encodings | HQL-0041 part 3 | gen-purpose-leap | none |
| `Bytes` and module `bytes` | HQL-0053 part 1 | gen-purpose-leap | one line in [Formats](../public/learn/09-io/formats.md) |
| `stats`: means, spread, quantiles, mode, correlation | HQL-0041 part 2 | stats-env | none |
| `io.env`: environment variables and `--arg` | HQL-0041 part 5 | stats-env | none |
| `time`: `Date`, `DateTime`, `Instant`, `Duration` | HQL-0041 part 1 | main | [Date and time](../public/learn/02-data-types/time.md), current |
| Analytic windows | HQL-0050 | main | [Windows](../public/learn/20-olap/windows.md), current |

Keys, signatures, MACs, password hashing, encryption and the key vault
(HQL-0053 parts 2 to 5) are accepted and not implemented.

## Book plan

The three parts stay: Part I the language, Part II data flows, Part III
knowledge and AI. New pages go in the chapter whose subject they extend.
A chapter is added only where none fits.

Part I, the language:

- **2. Data types** gains **Bytes**: a scalar of octets, its hex and base64
  forms, `xs[i]` and `xs[a:b]` through `Index<Int>` and `Slice`, order, and
  `to_bytes` and `canonical`.
- **9. I/O** gains **Files and folders** (`io.fs`: a `Path`, walking a
  `Folder`, matching a `Glob`, reading an `Entry`) and **Environment and
  arguments** (`io.env`, `--arg`, and what a host grants).
- A new chapter, **The standard library**, after Errors, holds the modules
  that are neither a type nor I/O:
  - **Statistics**: `stats` over a list and over a query's column.
  - **Random values**: sampling, shuffling, choice and UUIDs, and the seed
    fixed per evaluation.
  - **Digests and fingerprints**: `crypto.digest`, its algorithms, and the
    fingerprint of a `Data` tree over canonical JSON.
  - **Text and byte encodings**: `encoding`, between `String` and `Bytes`.
  - **Keys, signatures and encryption**: HQL-0053 parts 2 to 5, opening
    with the "Design only" warning until they ship.

  Working with hql follows it, and the chapter numbers of Parts II and III
  move by one. The files under `doc/public/learn/` are renamed with them.
- **10. The type system**: [Traits](../public/learn/10-the-type-system/traits.md)
  has its "In short" section; [Operators](../public/learn/10-the-type-system/operators.md)
  lists `Bytes` among the types `Index` and `Slice` serve.
- **13. Working with hql** is checked against HQL-0025 as implemented.

Part II, data flows:

- **20. OLAP**, [Deduplication](../public/learn/20-olap/deduplication.md),
  deduplicates by fingerprint where a row has no key.
- **14. ETL pipelines** reads its settings from `io.env` where it reads
  them from the command line today.

Every new code block runs under the book's runner, as the existing pages do.

## The crypto demo in `hql-demo`

A folder `crypto/` in [`hql-demo`](https://github.com/ewiger/hql-demo),
in the repository's form: an `hql.toml`, a `run.sh` compared against
`expected/`, a row in the README's table and a line in `run-all.sh`. It
needs no service.

1. **Manifest**: walk a folder with `io.fs`, and write a manifest of each
   file's path, size and SHA-256 digest in hex.
2. **Verify**: read the manifest back against the folder after `run.sh`
   alters one file and removes another, and report each file as unchanged,
   changed or missing.
3. **Fingerprints**: fingerprint the records of `events.json`, so that two
   records that differ only in key order or whitespace share one fingerprint.
   Deduplicate by it.
4. **Encodings**: the same digest in hex and base64, and a `Bytes` value
   that goes out and comes back unchanged.
5. **Signed manifest**, once HQL-0053 parts 2 and 3 ship: sign the manifest
   with an Ed25519 key read from the environment through `io.env`, and verify
   the signature before the check in step 2.

## Order of work

1. Merge `feat/stats-env` and `feat/gen-purpose-leap` into `main`.
2. On a branch from `main`, write the Part I pages, then the Part II
   additions, then the chapter renumbering, committing each part on its own.
3. The crypto demo, steps 1 to 4, in `hql-demo`. Step 5 follows HQL-0053
   parts 2 and 3.

## Done when

- Every feature in the table has a book page or section that runs under
  the book's runner, and `mkdocs build --strict` passes.
- The unshipped HQL-0053 parts are in the book behind the "Design only"
  warning.
- `hql-demo/crypto/` runs steps 1 to 4 from `run-all.sh` and matches
  `expected/`.
