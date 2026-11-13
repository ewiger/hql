# HQL-0041: Time, statistics, digests and encodings, randomness, and the environment

**Status**: implemented
**Created**: 2026-10-24
**Source**: a comparison of `lib/std/` against Python's standard library
(`datetime`, `statistics`, `hashlib` and `base64`, `random`, `os.environ` and
`sys.argv`); the non-goals of [HQL-0040](../HQL-0040/README.md), which defer
file times to a timestamp type

## Companion notes

- [`parts/01-time.md`](parts/01-time.md) — module `time`: instants, local
  date-times, zones, durations, calendar arithmetic, parsing and rendering,
  and the evaluation clock.
- [`parts/02-stats.md`](parts/02-stats.md) — module `stats`: means, spread,
  medians, quantiles, the mode, covariance and correlation.
- [`parts/03-digest-and-encoding.md`](parts/03-digest-and-encoding.md) — modules
  `crypto.digest` and `encoding`: digests of text and of `Data`, and the text
  encodings hex, base64 and percent-encoding.
- [`parts/04-random.md`](parts/04-random.md) — module `random`: sampling,
  shuffling, choice, numbers and UUIDs, seeded and reproducible.
- [`parts/05-env.md`](parts/05-env.md) — module `io.env`: environment
  variables and program arguments.

## Abstract

Five standard modules join `lib/std/`: `time`, `stats`, `crypto.digest`
with `encoding`, `random`, and `io.env`. `time` gives the existing `Date` its
operations and adds `Instant`, `DateTime`, `Zone` and `Duration`. `stats` adds
the descriptive statistics as reductions over numeric collections.
`crypto.digest` gives digests of a `String` and of a `Data` tree; `encoding` converts text to
and from hex, base64 and percent-encoding. `random` samples, shuffles and
draws numbers and UUIDs, pure when a seed is given and effectful otherwise.
`io.env` reads environment variables and the arguments an invocation passes.
Every effectful operation here reads a value fixed once per evaluation: the
clock, the zone, the generator's seed, the environment and the arguments. The internal
`std` extension implements all of them, and none of them is in the prelude.

## Motivation

- `core` declares `Date`, and no literal, constructor or operation produces
  or reads one. Card metadata such as `date: 2026-10-24` is a `String`, so
  sorting, filtering or grouping cards by time compares text.
- HQL-0040 leaves file modification times out until a timestamp type exists.
- `collections` has `sum`, `min`, `max` and `count`, and no mean, median,
  spread or quantile. Each is a `reduce` a program writes by hand, and the
  non-mergeable ones (median, quantiles, mode) cannot be split correctly by
  an executor that does not know what they are.
- A program cannot derive a stable identifier from content, detect duplicate
  records by digest, or read and write base64 and percent-encoded fields
  found in JSON and URLs.
- A program cannot take a sample of a large vault or stream, which is the
  ordinary first look at data, or produce one reproducibly.
- A program cannot be parameterised: every value it uses is written into its
  source, so the same query cannot run against two inputs without editing it.

## Non-goals

- Literal syntax for time values. Time values are built by constructors from
  ISO 8601 text; a literal form is separate grammar work.
- Operator syntax across time types. `Add` and `Sub` are `Self` by `Self`
  ([`lib/std/core/mod.hql`](../../../lib/std/core/mod.hql)), so `Duration`
  takes `+` and `-`, and an instant moves by a duration through named
  operations.
- Leap seconds, calendars other than the proleptic Gregorian, and
  sub-nanosecond precision.
- A `Bytes` type and binary digests or encodings of anything but text.
  Binary values belong to [issue 0029](../../issues/0029-binary-formats-and-codecs.md);
  until then every input here is a `String`, which is UTF-8.
- Cryptography: signing, encryption, keyed MACs and password hashing.
- Writing to the environment, spawning processes, and a command-line
  argument parser with flags, types and help text.
- Inferential statistics, distributions and regression.

## Specification

### 1. Ownership

Each module imports `core` and nothing else, apart from `io.env`, which
imports `core` and `io`.

| Module | File | Holds |
| --- | --- | --- |
| `time` | `lib/std/time/mod.hql` | `Zone`, the units, the operations over the time types, the clock |
| `stats` | `lib/std/stats/mod.hql` | the statistical reductions |
| `crypto.digest` | `lib/std/crypto/digest/mod.hql` | `Algorithm`, the algorithms, `digest`, `fingerprint` |
| `encoding` | `lib/std/encoding/mod.hql` | `Encoding`, the encodings, `encode`, `decode` |
| `random` | `lib/std/random/mod.hql` | sampling, shuffling, choice, numbers, UUIDs |
| `io.env` | `lib/std/io/env/mod.hql` | `var`, `vars`, `arg`, `args` |

- `Instant`, `DateTime` and `Duration` MUST be declared in `core`, beside
  `Date`, as `{Scalar, Orderable}`. A `Data` tree holds a scalar, and a
  format encodes one, so the scalar types sit where `Data` and `format`
  see them. `Zone` and the operations are `time`'s.
- The modules MUST be listed in `library::MODULES` after `core` and before
  any module that imports them; `io.env` MUST follow `io`.
- None of them is in the prelude, and none MUST be added to it. A program writes `import time`,
  `import stats`, `import crypto.digest`, `import encoding`, `import random`
  or `import io.env`, and reaches the last two as `digest` and `env`. The unit names (`Day`,
  `Week`) and the algorithm names (`Sha256`) are short and common, and the
  prelude stays the list of what every program needs.
- The `std` extension's descriptor MUST provide each module.

### 2. Values fixed per evaluation

An effectful operation in these modules reads one of five values, each fixed
when an evaluation starts and held until it ends.

| Value | Read by | Fixed from |
| --- | --- | --- |
| the evaluation instant | `time.now`, `time.today` | the host clock, or `--now <instant>` |
| the evaluation zone | `time.local_zone`, `time.today()` | `--zone`, else `TZ`, else the host's zone |
| the evaluation seed | the unseeded operations of `random` | the host's entropy source, or `--seed <int>` |
| the environment | `env.var`, `env.vars` | the process environment, filtered by the host grant |
| the arguments | `env.arg`, `env.args` | `--arg name=value`, repeatable |

- Each value MUST be read once per evaluation. Two calls of `now()` in one
  evaluation return the same instant, and a cell re-run in one evaluation
  sees the environment it saw first.
- An evaluation MUST report the seed it used in its run record, so a run with
  an entropy seed can be repeated with `--seed`.
- With `--now`, `--zone`, `--seed` and the same `--arg`s, environment and inputs, an
  evaluation MUST produce the same value. This is what makes a test of a
  program that reads the clock or draws a sample deterministic.

### 3. No silent coercion

A value of a new type enters a program only through a constructor or a
conversion it writes.

- A decoder MUST NOT infer a time value from text. YAML's timestamps and
  JSON's date-looking strings decode as `String`; `s.to_date()` or
  `Instant(s)` converts them, and so does a declared field of a time
  type, whose declaration is the conversion.
- An encoder MUST write a time value as its ISO 8601 text (RFC 3339 for an
  `Instant`), and `Duration` as its ISO 8601 duration text.
- A constructor applied to text that does not parse MUST be an evaluation
  error naming the text; the `to_*` conversion of the same text gives `none`.
  This mirrors `Data(s, Json)` and `String.to_int`.

## Backwards Compatibility

- Additive. `Date` keeps its declaration, its `Eq` and its `Ord`; it gains
  operations in `time`.
- `core` gains three scalar types. A program that declares a type named
  `Instant`, `DateTime` or `Duration` collides with `core`, which every
  module imports, and renames its own.
- HQL-0040's `Entry` MAY gain `modified? : Instant` once `time` lands; its
  non-goal on file times is lifted by this proposal and changed there.
- `env` becomes a local name in a unit that imports `io.env`, as `fs` does
  for `io.fs`, and `digest` in a unit that imports `crypto.digest`.

## Security Considerations

- The environment commonly holds credentials. `io.env` reads only the names
  the host grants (see [`parts/05-env.md`](parts/05-env.md)), and under
  `hql render` it grants none.
- `random` is not a source of secrets. Its generator MUST be documented as
  unfit for keys or tokens, and a seeded draw is predictable by design.
- `crypto.digest` offers `Md5` and `Sha1` for interoperability with existing
  digests, and both are documented as broken for collision resistance.
- `fingerprint` and `digest` read their whole input. They are bounded by the
  run-context limits of [HQL-0012](../HQL-0012/README.md) as any held value
  is.
- Time-zone data is compiled into the binary, and the host's database is
  read instead where one is installed. A program's results for future
  instants in a zone can change when that database changes; the binary MUST
  report in `hql --version` which database it resolves zone names against:
  `system` for the host's, or the release of the one it carries.

## Reference Implementation

- `lib/std/core/mod.hql`: `Instant`, `DateTime`, `Duration` and their `Eq`,
  `Ord`, and `Duration`'s `Add` and `Sub` implementations.
- `lib/std/time/`, `stats/`, `crypto/digest/`, `encoding/`, `random/` and `io/env/`:
  one `mod.hql` each, written as external declarations.
- `src/extensions/std/`: the implementations, one file per module.
- Crates, each removing work the standard library does not do: `jiff` for
  zones, the time-zone database and calendar arithmetic; `sha2`, `sha1`,
  `md-5` and `blake3` for digests; `base64` for base64;
  `rand_chacha` for the seeded generator; `uuid` for UUID layout.
  Each is added with `cargo add` per [the stack](../../stack.md).
- `src/main.rs`: `--now`, `--zone`, `--seed` and `--arg`.

## Test Plan

- Each part names its own cases. Across all of them:
- One proposed example program per module under `examples/`, with
  `status: proposed` and `proposal: HQL-0041`, promoted as each checks.
- Determinism: a program reading `now()`, sampling unseeded and reading an
  argument gives the same value twice under fixed `--now`, `--seed` and
  `--arg`, and reports its seed when `--seed` is absent.
- Coercion: a JSON and a YAML document holding a timestamp decode it as a
  `String`; encoding each time type writes its ISO 8601 text.
- Binding: every new module checks with `hql check`, and
  `tests/declarations.rs` walks it.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Changelog

- 2026-10-24: drafted
- 2026-11-12: part 1, `time`, implemented (issue 0064); `Unit` is
  `TimeUnit`, `Zone` is not `Orderable`, the zone defaults are overloads,
  and `abs` over `Duration` waits for `math` to take it
- 2026-11-13: accepted
- 2026-11-13: part 4, `random`, implemented (issue 0066); each form of a
  name states its own purity, a call site is the call's byte offset in the
  program text, the evaluation seed is reported as an `info` diagnostic,
  `uuid(name, namespace)` also takes `"dns"`, `"url"`, `"oid"` and
  `"x500"`, and localhost runs one partition
- 2026-11-13: part 3's digest module is `crypto.digest`, and its algorithms
  are cryptographic hash functions only
- 2026-11-13: part 3, `crypto.digest` and `encoding`, implemented; the
  algorithms and encodings are built-in type names, and canonical JSON
  writes a number as its exact digits without an exponent
- 2026-11-13: part 2, `stats`, implemented; over `Int` and `Decimal` values
  a reduction's state is `(n, Σx, Σx²)`, exact and combined by addition,
  which is Chan's update in exact arithmetic, and over `Float` values it is
  `(n, mean, M2)` combined by Chan's update; a `Decimal` root is rounded
  from the exact ratio; `quantile` also takes a `Decimal` `q`, a literal
  being a `Decimal`, and a `Float` `q` is read by its shortest digits over
  exact values; `quantiles` computes each cut point with one division,
  Python's inclusive method; a `Float` is finite, so no input holds NaN or
  an infinity, and a statistic past the `Float` range is an overflow error;
  with `stats` imported, `variance` over `Int` or `Float` values is
  ambiguous with the prelude's population `variance` and is written
  `stats.variance`; localhost runs one partition
- 2026-11-13: part 5, `io.env`, implemented; the environment is read from
  the process once and held for the process, the grant filtering it as a
  program reads it; a name or value that is not UTF-8 is not held; `--arg`
  without `=` is refused as a malformed name is, exit 2
- 2026-11-13: part 1's remainder (issue 0064): `math` declares `abs` over
  `Duration`; HQL-0040's `Entry` gains `modified? : Instant`; and
  `hql --version` names the time-zone database zone names resolve against,
  `system` where jiff finds the host's and the bundled release otherwise,
  since the host's database is preferred where one is installed
