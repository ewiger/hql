# 0064 — `Date` as a data type, and the `time` module

Status: doing
Related: [HQL-0041](../proposals/HQL-0041/README.md),
[HQL-0041 part 1](../proposals/HQL-0041/parts/01-time.md),
[HQL-0040](../proposals/HQL-0040/README.md),
[`lib/std/core`](../../lib/std/core/mod.hql),
[the data-type card](../wiki/hql/types/data-type.hmd)

`Date` is declared in `core` with native `Eq` and `Ord`, and nothing produces
one: no literal, no constructor, no parser. SQL `DATE` and `TIMESTAMP` columns
cross the boundary as text. HQL-0041 part 1 gives `Date` its operations and
adds `DateTime`, `Instant` and `Duration` to `core`, with `Zone`, the units,
and every operation in a new `time` module that the internal `std` extension
implements. This issue carries that part into code.

## Work

1. `lib/std/core/mod.hql`: `DateTime`, `Instant` and `Duration` declared beside
   `Date`, with `Eq` and `Ord` for each and `Add` and `Sub` for `Duration`.
2. `lib/std/time/mod.hql`: module `time` with `Zone`, the `Unit` hierarchy,
   and the signatures of part 1: construction and conversion, zones, parts,
   arithmetic, truncation and ranges, rendering, and the clock.
3. The runtime: a value for each of the four scalars, built on `jiff`, with
   the precision and range part 1 requires.
4. The `std` extension: the natives behind every `time` signature; `now()`,
   `today()` and the zone read once per evaluation.
5. The boundary: SQL `DATE`, `TIMESTAMP` and `TIMESTAMPTZ` columns read as
   `Date`, `DateTime` and `Instant` instead of text, and written back the
   same way.
6. Tests: the test plan of part 1, as checked examples under `examples/` and
   as integration tests under `tests/`.
7. Cards and models: the data-type card no longer says `Date` has no literal
   and open operations; the operation-signatures model gains the `time`
   signatures; the learn page on data types gets `Date` in its table and a
   page of its own.
8. HQL-0041 changelog: part 1 marked implemented.

## State

Items 1 to 4, 6 and 7 are in, with the boundary of item 5 for the tree
formats, CSV and SQLite, and the PostgreSQL and Spark column types mapped
without a live server to run against. A declaration with no positional
parameter takes `Unit` as its input, which `now()`, `today()`,
`local_zone()` and the named-part constructors need.

Open:

- PostgreSQL `DATE`, `TIMESTAMP` and `TIMESTAMPTZ`, and Spark `DATE`,
  `TIMESTAMP_NTZ` and `TIMESTAMP`, read and written against a server.
- `abs` over `Duration`: `math` declares `abs`, and a second module
  declaring it is not yet resolved.
- `hql --version` naming the time-zone database it carries.
- HQL-0040's `Entry` gaining `modified? : Instant`.
- `g.key` of a `group` over a record type the program declares is refused
  as an unknown field, whatever the key's type.
