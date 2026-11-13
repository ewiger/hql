# 0070 — The HQL book catches up with the general-purpose leap

Status: done
Related: [0054](../0054-user-facing-website.md),
[0067](../0067-std-crypto.md),
[HQL-0025](../../proposals/HQL-0025/README.md),
[HQL-0039](../../proposals/HQL-0039/README.md),
[HQL-0040](../../proposals/HQL-0040/README.md),
[HQL-0041](../../proposals/HQL-0041/README.md),
[HQL-0053](../../proposals/HQL-0053/README.md)

Branches `feat/gen-purpose-leap` and `feat/stats-env` implement a run of
standard modules and language features that the reference pages document
and the book does not. A reader of [the HQL book](../../public/learn/index.md)
learns none of `Bytes`, files and folders, random values, statistics,
digests, encodings or the environment. The book is the one place that
teaches them in order. This issue brings it level with the code, keeping
its three parts, and adds a crypto demo to `hql-demo`.

## What shipped, and where the book stands

| Feature | Record | Branch | Book today |
| --- | --- | --- | --- |
| Traits: validator traits, `trait A as F`, module trait lists | HQL-0039 | gen-purpose-leap | [Traits](../../public/learn/10-the-type-system/traits.md), current |
| Execution contexts, named contexts, `HQL_PATH` | HQL-0025, in part | gen-purpose-leap | [Working with hql](../../public/learn/13-working-with-hql.md), to be checked |
| `io.fs`: `Path`, `Folder`, `Glob`, `Entry` | HQL-0040 | gen-purpose-leap | none |
| `random`: sampling, shuffling, numbers, UUIDs | HQL-0041 part 4 | gen-purpose-leap | none |
| `crypto.digest`: digests and fingerprints | HQL-0041 part 3 | gen-purpose-leap | none |
| `encoding`: hex, base64 and the other text encodings | HQL-0041 part 3 | gen-purpose-leap | none |
| `Bytes` and module `bytes` | HQL-0053 part 1 | gen-purpose-leap | one line in [Formats](../../public/learn/09-io/formats.md) |
| `stats`: means, spread, quantiles, mode, correlation | HQL-0041 part 2 | stats-env | none |
| `io.env`: environment variables and `--arg` | HQL-0041 part 5 | stats-env | none |
| `time`: `Date`, `DateTime`, `Instant`, `Duration` | HQL-0041 part 1 | main | [Date and time](../../public/learn/02-data-types/time.md), current |
| Analytic windows | HQL-0050 | main | [Windows](../../public/learn/20-olap/windows.md), current |

| Keys, their formats and the secret types | HQL-0053 part 2 | gen-purpose-leap | none |

Signatures, MACs, password hashing, encryption and the key vault
(HQL-0053 parts 3 to 5) are accepted and not implemented.

## Book plan

Parts I to III keep their chapters and numbers. New pages go in the chapter
whose subject they extend, and the standard library modules that are
neither a type nor I/O form a new **Part IV: The standard library**, after
Part III, so no chapter is renumbered. Part IV takes further topics as the
library grows.

Part I, the language:

- **2. Data types** gains **Bytes**: a scalar of octets, its hex and base64
  forms, `xs[i]` and `xs[a:b]` through `Index<Int>` and `Slice`, order, and
  `to_bytes` and `canonical`.
- **9. I/O** gains **Files and folders** (`io.fs`: a `Path`, walking a
  `Folder`, matching a `Glob`, reading an `Entry`) and **Environment and
  arguments** (`io.env`, `--arg`, and what a host grants).
- **10. The type system**: [Traits](../../public/learn/10-the-type-system/traits.md)
  has its "In short" section; [Operators](../../public/learn/10-the-type-system/operators.md)
  lists `Bytes` among the types `Index` and `Slice` serve.
- **13. Working with hql** is checked against HQL-0025 as implemented.

Part II, data flows:

- **20. OLAP**, [Deduplication](../../public/learn/20-olap/deduplication.md),
  deduplicates by fingerprint where a row has no key.
- **14. ETL pipelines** reads its settings from `io.env` where it reads
  them from the command line today.

Part IV, the standard library:

- **28. Statistics**: `stats` over a list and over a query's column.
- **29. Random values**: sampling, shuffling, choice and UUIDs, and the
  seed fixed per evaluation.
- **30. Digests and encodings**: `crypto.digest`, its algorithms, the
  fingerprint of a `Data` tree over canonical JSON, and `encoding` between
  `String` and `Bytes`.
- **31. Keys and cryptography**: keys, their formats and the secret types;
  signatures, encryption and the key vault behind the "Design only" warning
  until they ship.

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

1. Merge `feat/stats-env` into `feat/gen-purpose-leap` (done).
2. On `feat/gen-purpose-leap`, write the Part I pages, the Part II
   additions and Part IV, committing each part on its own.
3. The crypto demo, steps 1 to 4, in `hql-demo`. Step 5 follows HQL-0053
   parts 2 and 3.

## Done when

- Every feature in the table has a book page or section that runs under
  the book's runner, and `mkdocs build --strict` passes.
- The unshipped HQL-0053 parts are in the book behind the "Design only"
  warning.
- `hql-demo/crypto/` runs steps 1 to 4 from `run-all.sh` and matches
  `expected/`.

## Outcome

- Part I: [Bytes](../../public/learn/02-data-types/bytes.md),
  [Files and folders](../../public/learn/09-io/files-and-folders.md) and
  [Environment and arguments](../../public/learn/09-io/environment.md);
  [Operators](../../public/learn/10-the-type-system/operators.md) lists
  `Bytes` under positions; [Working with hql](../../public/learn/13-working-with-hql.md)
  states the context behaviour that ships.
- Part II: a setting read with `--arg` in
  [ETL pipelines](../../public/learn/14-etl-pipelines.md), and duplicates with
  no key kept by fingerprint in
  [Deduplication](../../public/learn/20-olap/deduplication.md).
- Part IV, the standard library: [Statistics](../../public/learn/28-statistics.md),
  [Random values](../../public/learn/29-random.md),
  [Digests and encodings](../../public/learn/30-digests-and-encodings.md) and
  [Keys and cryptography](../../public/learn/31-cryptography.md). Keys,
  signatures, MACs and password hashes run; encryption and the key vault
  are behind the "Design only" warning.
- `hql-demo`: `crypto/` runs steps 1 to 5 on branch `crypto-release`,
  the signed manifest included, since HQL-0053 part 3 shipped.

The language gaps the pages met are not the book's work and stay here until
an issue takes them:

- `Map<String, String>` from `env.args()` and `env.vars()` takes only `get`;
  `size`, `keys` and `m["k"]` are check errors, and `entries` gives `Pair`
  with `first` and `second`.
- `some(x)` prints as `x`; `none` prints as `empty` inside a record and as
  `none` inside a list.
- A literal `0.0` is a `Decimal`, so a `Float` result defaults with `0.0f`.
- A list of record literals with a `Data` field is inferred as `Data`.
- A typed read of one JSON array, `Seq<List<T>>`, is refused; there is no
  step from a `Seq` to a `List`.
- `file(p) | text` gives the endpoint, not the file's text.
- A public key formatted as JSON is its display text, not its JWK.
- The checker accepts an effectful draw, `random_int`, in a function not
  declared `effectful`.
- The message "a Aes256Gcm key"; "the DER text" for a PEM input; a caret
  one column short under a call's closing parenthesis in the
  `hash_password` cost error.
- `examples/crypto/keys/rfc8032-test-1.pem` is checked out with mode 0644,
  so reading it warns.
