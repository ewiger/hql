# 0053 — Bugs found reviewing the complete type system branch

Status: doing
Related: [issue 0052](0052-review-follow-ups.md),
[HQL-0031, Decimal](../proposals/HQL-0031/README.md),
[HQL-0035, schema boundaries](../proposals/HQL-0035/README.md)

The line-by-line review that issue 0052 left due, over the checker, the parser,
the collection steps, the schema boundary and the sources. Every item below was
reproduced against the merged `main`; each states the input and the observed
and expected result.

## Parser

1. **An argument list cut off at the end of input panics.** `combiner`
   (`src/parser.rs`) indexes the token after `End`. `max(` and `max(1,` panic
   with "index out of bounds". Older than the branch.
2. **A `fork` branch holds only plain steps.** `fork` parses each step with
   `link`, not the step dispatch `pipeline` uses, though
   [pipes](../wiki/hql/types/pipes.hmd) says every branch is a pipeline.
   `[1,2,3] | fork { a = take(2) | List<Int> }` fails with "comparisons do not
   chain"; a nested `fork` in a branch fails with "expected a branch name".
3. **`fork` branches need no separator.** `fork { a = take(2) b = take(1) }`
   is accepted; a line break or a comma is required.
4. **A destructuring pattern cannot break lines or end in a comma.**
   `destructured_names` reads raw tokens. `{a,\n b} = P(a = 1, b = 2)` and
   `{ a, b, } = …` fail with "expected end of statement, found `=`".
5. **`import` and `impl` are no longer usable as names.** `import = 5` then
   `import` fails with "expected a module name"; `impl = 2` then `impl < 3`
   fails with "expected a type parameter". Both evaluated before the branch;
   [naming](../wiki/hql/naming.hmd) recognises them by position.
6. **`type X =` and a line break refuse a structural body.** `type X =\n{ a: Int }`
   fails with "expected `union` after `=`", while the same break before
   `union { … }` parses. `type X : Data =\n{ a: Int }` fails too.
7. **Empty brackets lose their message across a line break.** `xs[ ]` gives
   the dedicated message; `xs[\n]` gives "expected an expression".

## Checker

8. **An operator matches a trait function by name only.** `method_for`
   (`src/checker.rs`) takes any `add` for `a + b`. With `impl Foo for V`
   declaring `add`, `v + v` checks and fails at evaluation with "no
   implementation of `Add`"; with both `Foo` and `Add` implemented, a valid
   `v + v` is refused as ambiguous. `==` and `eq` share the lookup.
9. **A trait function's `Self` is the receiver's type, not the implementing
   type.** With `impl Fresh for Base` and `fn fresh(self) : Self`,
   `s.fresh().y` on a `Sub` checks as `Int` and fails at evaluation with "Base
   has no field y". The same rule refuses `s == Base(x = 1)` under
   `impl Eq for Base`, while `Base(x = 1) == s` is accepted.
10. **A generic record widens a parameter without re-checking earlier
    fields.** `construct_record` through `unify`.
    `Box(f = (n: Int) => n + 1, x = "s")` is typed `Box<Data>`; `b.f(b.x)`
    checks and fails at evaluation. A related root outside the checker:
    `TypeRef::is` treats a generic record as covariant in a parameter that
    sits in argument position.
11. **A user `Eq` is skipped when the left side is an `Option`.** With an
    `impl Eq` that always answers `true`, `a == b` is `true` and
    `some(a) == b` is `false`.
12. **`match none { none => 1 }` is refused.** Exhaustiveness counts a
    `Never` side as covered for `Result` but not for `Option`.
13. **The parameter-limit message says eight.** The limit is four; the text
    appears at every site that enforces it.

## Collections

14. **`take` on a `Set` ignores a user `impl Ord`.** `eval_take`
    (`src/extensions/collections.rs`) keeps storage order. `Set` of versions
    `| take(2)` gives `3.0.0, 1.10.0`, expected `0.9.9, 1.10.0`; `sort`,
    `min`, `max` and `top_k` already order by the trait.
15. **`approx_distinct` counts equal values twice.** `encode` tags `Int` and
    `Decimal` apart and hashes the type of an absent value.
    `[1, 1.0] | approx_distinct` gives `2`; `Set([1, 1.0]) | size` gives `1`.
16. **`join` and `semi_join` match keys by structure, not `==`.** `eval_join`.
    Keys equal under a user `impl Eq` never match; the same join written with
    `on = (x, y) => x == y` does.
17. **`count_by` and `histogram` keep equal keys apart.** `tally`.
    `[none, some(1) | filter(x => x > 5)] | count_by(x => x)` gives
    `Map {none: 1, none: 1}`, expected `Map {none: 2}`. `group` likely shares
    the defect.
18. **`collect`, `emit` and `flat_map` refuse a finite iterating function.**
    `few() | collect(2)` over `yield 3 yield 1 yield 2` fails with "needs a
    collection", while `take`, `drop`, `sum` and `sort` accept it. Older than
    the branch.
19. **`count(x)` and `contains(x)` ignore a user `impl Eq`.** `count(P(1, 9))`
    gives `0` where `filter(p => p == P(1, 9))` finds one. Older than the
    branch.

## Boundary and sources

20. **A `Bool` written to SQLite does not read back.** It is stored as
    `INTEGER 1`, and the read refuses "1 is not Bool" though the type table
    decodes `INTEGER` to `Bool`.
21. **An integer is refused as `Float`.** JSON `{"x": 3}` and an SQLite
    `INTEGER` column read into `x: Float` fail with "3 is not Float"; the type
    table decodes both to `Float`.
22. **A `Decimal` read from `NUMERIC` loses digits without `lossy = true`.**
    `sqlite_columns` (`src/sources.rs`) returns names only, so the declared
    type is never checked. `12345678901234567.89` reads as
    `12345678901234568`. The read-side twin of the write defect in issue 0052.
23. **Views and `WITHOUT ROWID` tables cannot be read.** With no `order`
    configured the read sorts by `rowid`: "no such column: rowid".
24. **Non-UTF-8 `BLOB` bytes become U+FFFD.** The cell goes through
    `String::from_utf8_lossy`, and `BLOB` is not in the lossy table.
25. **An unreadable `hql.toml` reads as no configuration.** `Sources::read`
    treats invalid UTF-8 or a permission failure like a missing file, so every
    source reports "no source is configured". Only `NotFound` means no sources.
26. **A flat name and a path with the same text collide.**
    `fields.flat.name = "a.b"` and `fields.deep.path = "a.b"` are refused as a
    shared external field; the check compares the written text.
27. **SQLite column names compare case-sensitively.** `ID` and `Name` columns
    read into `id` and `name` report unmapped fields.
28. **A record with no writable field gives a raw SQL error.** Writing it
    fails with `near ")": syntax error` instead of a boundary error.
29. **Two fields that swap external names lose a value.** `remapped`
    (`src/extensions/io.rs`): `x.name = "y"` and `y.name = "x"` give "field x:
    it is missing".
30. **A `path` field breaks a closed record.** The path's top-level container
    is never removed, so `type P { id: String, name: String }` read from the
    station example fails with "P declares no such field profile", even under
    `strictness = "open"`.

## Work plan

Each unit is one commit, made once `cargo test` and `cargo clippy` pass, and
carries a regression test for every item it closes. A unit is ticked here in the
commit that closes it. Issue 0052's items are folded in as the last units.

- [x] U1 — parser: an argument list cut off at the end of input (1)
- [x] U2 — parser: `fork` branches are pipelines and need a separator (2, 3)
- [x] U3 — parser: destructuring across lines and with a trailing comma (4)
- [x] U4 — parser: `import` and `impl` usable as names (5)
- [x] U5 — parser: `type X =` and a line break; empty brackets (6, 7)
- [x] U6 — checker: operators dispatch through their own trait (8)
- [x] U7 — checker: `Self` is the implementing type (9)
- [x] U8 — checker: a widened record parameter re-checks its fields (10)
- [x] U9 — checker: `Option` equality, `match none`, parameter limit text (11, 12, 13)
- [x] U10 — collections: HQL equality in `approx_distinct`, `join`, `tally`, `group`, `count`, `contains` (15, 16, 17, 19)
- [x] U11 — collections: `take` on a `Set` by its order (14)
- [x] U12 — collections: finite iterating functions in `collect`, `emit`, `flat_map` (18)
- [ ] U13 — SQLite read: `Bool`, `Float` from integers, declared `Decimal` columns, no `rowid`, `BLOB` (20–24)
- [ ] U14 — sources: an unreadable `hql.toml` is an error (25)
- [ ] U15 — boundary: name and path collision, column case, no writable column (26, 27, 28)
- [ ] U16 — io: swapped external names, `path` in a closed record (29, 30)
- [ ] U17 — SQLite write: a `Decimal` keeps its digits (0052, item 1)
- [ ] U18 — `merge` and `distinct` in linear time (0052, items 2 and 3)
- [ ] U19 — `decimal.rs` cleanup (0052, item 4)
