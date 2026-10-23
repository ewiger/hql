# 0045 — Dataflow, errors as values, validator types and schema boundaries

Status: in progress
Related: [HQL-0032](../proposals/HQL-0032/README.md),
[HQL-0033](../proposals/HQL-0033/README.md),
[HQL-0034](../proposals/HQL-0034/README.md),
[HQL-0035](../proposals/HQL-0035/README.md),
[HQL-0036](../proposals/HQL-0036/README.md),
[HQL-0037](../proposals/HQL-0037/README.md),
[0043](0043-contract-validation.md),
[0044](0044-typed-tables.md),
[the structural examples](../../examples/structural/README.md)

The next portion of the type system after contract rules,
`x | validate(T)`, `Table<R>` and `Decimal`. It is a list of work items for
an implementation loop, ordered by dependency: one item is finished and
committed before the next starts.

## Loop protocol

Each item runs the same cycle.

1. Run `git status doc/proposals/` and re-read any proposal that changed: the
   proposals are edited while the loop runs, and an edit is followed as
   written.
2. Write the item's cases under `examples/` first, headed
   `status: proposed` with the `proposal:` they come from, and add them to
   the folder's README table. The corpus test asserts that a proposed case
   does not check yet.
3. Implement until each case checks, then promote it to `valid-now` with the
   verified `expected-type` and `expected`.
4. `cargo test` and `cargo clippy --all-targets` pass with no warnings.
5. Update the proposal's status line and changelog, the
   [proposal index](../proposals/README.md), `CHANGELOG.md`, the wiki card
   that states the feature, and this issue's Progress.
6. Commit the item on its own.

Where a proposal leaves a point as a placeholder that an item needs, the
item states it in the proposal first, as a short specification, and names
that in its commit. It does not add open questions.

## Work items

### The dataflow DAG ([HQL-0037](../proposals/HQL-0037/README.md))

- **D1 — `fork`.** `xs | fork { a = …, b = … }` parses as a structural
  form, every branch consumes the same upstream, and the result is a record
  of the branch results, each typed by its own pipeline. Destructuring
  `{ a, b } = xs | fork { … }` binds the branches. Each branch keeps the
  upstream's order.
- **D2 — Shared evaluation.** A bound producer used by several consumers is
  evaluated once: a live source under a `fork`, or bound once and read by
  two pipelines, is read once, and `max_elements` counts its elements
  once. A pure branch whose result nothing uses is not run; an effectful
  one is.
- **D3 — Fan-in over streams.** `merge`, `zip` and `concat` over two or more
  flows of compatible element types, with the ordering each one states.
  `zip` and `concat` exist over `Seq` and lists and extend to `Stream`.
  `merge` is overloaded for `Map` and `Stream` only, as HQL-0037's section
  on the overload states: the map combiner of
  [HQL-0014](../proposals/HQL-0014/README.md) keeps its meaning, a call is
  resolved by its argument types, and a named `merge` by the expected
  function type, so `maps | reduce({:}, merge)` still checks and `f = merge`
  is an ambiguous reference. Today the combiner is special-cased by name in
  `src/checker.rs` and `src/evaluation.rs`; it becomes an ordinary overload
  set. A `merge` over `List`, `Seq`, `Set` or `Table` is a type error naming
  `concat` or `union`.
- **D4 — Planner classification.** The plan in `src/backends/plan.rs` and
  `hql`'s plan output show fork, tee and fan-in nodes, and a `fork`'s
  branches as stages of one upstream.

### Errors as values ([HQL-0033](../proposals/HQL-0033/README.md))

`Result`, `map_err`, `oks()`, `errs()`, `Violation` and `Invalid` exist.

- **E1 — `Invalid` and `Violation` as the proposal declares them.**
  `Invalid` carries the original rejected `Data` as `value` and its
  `violations`; `Violation`'s `expected` and `actual` are filled; a stored
  `Invalid` reads back with `| Invalid`. `x | T` and `x | validate(T)`
  derive the same violations. Covers the test plan's cases on multiple
  violations, the original `Data`, fail-fast equivalence, and
  `flat_map(e => e.violations)`.
- **E2 — Mapping results inside streams.** `map`, `map_err` and `match` over
  each `Result` of a `List`, `Seq` and `Stream`, as the sections on handling
  one `Result` and on mapping results inside streams state; `oks()` and
  `errs()` over all three collection kinds.
- **E3 — Both sides with `fork`.** After D1 and D2:
  `fork { accepted = oks(), rejected = errs() }` over a `List`, `Seq` and
  `Stream` of `Result`, as the section on both sides with `fork` states.
  Every value lands on exactly one side, each side keeps source order, and a
  live source is evaluated once. There is no `split` over results.
- **E4 — Rejected values as ordinary data.** `errs()` or a `fork` branch
  written with `sink` and read back as `Invalid`; repair from `e.value` into
  another pipeline. Broker provenance (`Message<T>`) stays outside `Invalid`.
- **E5 — `retry`.** `retry` reinvokes a fallible operation under a policy;
  exhausted retries give an ordinary final `err(E)`. It never operates on a
  completed `Result`. State the policy's form in HQL-0033 first if the
  proposal leaves it open.
- **E6 — HQL-0032 aligned.** HQL-0032's conceptual `ValidationResult<T>` and
  its stream-routing items are marked superseded by HQL-0033 where they
  conflict; [0043](0043-contract-validation.md)'s stream policies are closed
  by E2 to E4. HQL-0033 is marked accepted once E1 to E4 are built.

### Validator types ([HQL-0034](../proposals/HQL-0034/README.md))

- **V1 — `Validator` and validator types.** `trait Validator` in `std/`; a
  type with `Validator` among its supertypes is a validator type;
  `validate f with V` and a field `f: V` hold where `V`'s `validate(self)`
  for the field's type is `true`. A type that does not implement it is a
  type error.
- **V2 — The placeholders.** State in HQL-0034, then build: the module that
  declares the standard validators and the first set (`Email`, `NonEmpty`,
  and others the examples need), a validator for more than one value type,
  and the identity and message of the `Violation` a validator type reports,
  consistent with the rule identities `as name` gives.

### Schema boundaries ([HQL-0035](../proposals/HQL-0035/README.md))

Scope is the kinds that exist: `csv`, `json`, `json_lines`, `yaml` and
`sqlite`. Registries, catalog providers other than SQLite's, Postgres,
refresh, caching, versioning and boundary events are not in this portion.

- **B1 — The configuration form.** State in HQL-0035 how a boundary is
  written in `hql.toml` under `[sources.<id>]`: per field an external name
  or path, an external type, a position, readable and writable, and a
  null or missing policy; extension-owned keys under the kind's prefix.
  Refused at load when malformed, like every other source key.
- **B2 — The resolver.** One resolver in `src/` merges format defaults,
  configuration and an invocation override by field-level recursive
  override into one effective boundary, keeping where each property came
  from. Boundary errors are structured diagnostics: a required field with no
  external column, two fields on one column, a configured column that does
  not exist, an external type with no decoder.
- **B3 — Reading through a boundary.** A CSV header, a JSON path such as
  `profile.display_name`, and a SQLite column map onto a declared field; the
  expected type decides which fields are read, and SQLite reads only those
  columns.
- **B4 — Nullability.** Each format's default for an optional field (an
  empty CSV cell, a missing JSON field, JSON `null`, SQL `NULL`) and a
  boundary override of it.
- **B5 — Representation overrides and loss.** A `Decimal` field written as a
  JSON string (left open in [0042](0042-decimal.md)); a lossy
  representation, such as `Decimal` as a JSON number, refused unless the
  boundary permits it.
- **B6 — Configured targets.** `target("id")` over a configured target,
  writing `Table<R>` and `Seq<R>` to CSV, JSON and SQLite through the same
  resolver: column order from `R` unless the boundary orders it, a field
  marked not writable left out. State the write modes (create, append,
  replace) in HQL-0035's Write modes placeholder first; closes
  [0044](0044-typed-tables.md)'s open item.
- **B7 — Strictness.** `strict`, `compatible`, `open` and `warn` for how
  external fields beyond the mapping are treated, without changing the
  openness of the HQL type.
- **B8 — Inspecting a boundary.** A CLI command prints a source's effective
  boundary: field, external field, both types, origin of each property, and
  read and write status.

### Partitioning ([HQL-0036](../proposals/HQL-0036/README.md))

- **P1 — `partition(by = …)`.** First the predicate
  `partition(self, by: T -> Bool) : Pair<List<T>, List<T>>` is removed from
  `std/seq.hql` and `src/extensions/seq.rs`; its use is a `fork` with two
  complementary filters. Then HQL-0036's form: type-preserving over `Seq`
  and `Stream`, logically `1 → 1`, one logical partition per key on the
  local backend, and a stage boundary in `src/backends/plan.rs`. A `fork`
  followed by `partition` stays two graph operations.

## Done when

- HQL-0033's test plan is covered by `tests/` and the corpus, and HQL-0033
  is accepted.
- HQL-0034 has no placeholder and its validator types are built.
- A CSV, JSON and SQLite source and target read and write a declared record
  through a configured boundary, and HQL-0035 states what is built.
- `fork` runs on the local backend with each producer evaluated once.
- `partition` has one meaning, and HQL-0036's form runs on the local backend.

## Progress

- E1 done: `expected` and `actual` filled by kind (stated in HQL-0033's
  `Violation` section), `Violation.severity` defaults to `"error"`, a
  library `DataContract` record such as `Invalid` reads from a tree, and
  `x | T` and `x | validate(T)` share one reading. Its cases checked at
  type level before the work, so they land as `valid-now` without a
  `proposed` step.
- E2 done: `oks()` and `errs()` over `Stream<Result<T, E>>` give a stream,
  lazily, through a traversal that keeps one side; `map`, `map_err` and
  `match` over each result already held for all three kinds and are pinned
  by a case.
- V1 done: `trait V : Validator`, trait extension with inherited functions,
  `validate f with V` dispatching to `V`'s implementation, and `f: V` as the
  one implementing type held to `V`. A validator type implemented for
  several types is refused as a field type until V2.
- D1 done: `fork` parses as a pipeline step, every branch reads the input
  evaluated once, and the value is a record type named by its branches,
  `{a: A, b: B}`; `{ a, b } = …` desugars to field reads of a hidden
  binding. Cases in `examples/dataflow/`.
- D2 done: a file read is a shared reading, opened on first use and
  replayed, like standard input; a configured source was already read once
  per evaluation, now pinned with `max_elements`. Lowering marks the pure
  branches of a bound `fork` that no later statement reads, and they are
  not run.
- E3 done: `fork { accepted = oks(), rejected = errs() }` needed no code
  beyond D1, D2 and E2; cases pin it over all three kinds, with source
  order, one side per value, a live file read once, and no `split` over
  results.
- E4 done: writing `errs()` and reading it back as `Invalid` worked after
  E1; pinned by a two-run test and a repair case. `Message<T>` does not
  exist, so HQL-0033 now states that provenance sits in an envelope beside
  `Invalid` until it does.
- P1 done: the predicate `partition` removed; `partition(by = …)` declared
  `partition fn` over `S : Seq<T>` and `Stream<T>`, a stage boundary in the
  plan, passing values on in order on the local backend through a lazy
  keyed traversal. HQL-0037 marked accepted.
- D3 done: `merge`, `zip` and `concat` over streams through lazy merge,
  zip and concat traversals that read their inputs a portion at a time;
  `merge` and `concat` also as steps. `merge`'s two forms are resolved by
  type, with the list, set and table refusals and the ambiguous reference;
  it stays in the core's combiner table, which `reduce`'s split relies on,
  as HQL-0037's changelog records.
- D4 done: planned steps carry a topology and a fan-out's branches; `hql
  plan` prints both and its JSON adds `topology` and `branches`. A combiner
  written as a step is a step node. Pinned by a CLI test; the plan is not a
  corpus value, so D4 has no corpus case.
- E5 done: HQL-0033's section on retry now states the form, `xs |
  retry(operation, attempts = n, when = …)`, elementwise and lazy over a
  stream, with no delay. Channels make the invocations countable in the
  case.
- E6 done: HQL-0032's runtime failure policies, stream validation,
  dead-letter handling and validation result, and its open questions on
  them, are marked superseded by HQL-0033; issue 0043's stream policies are
  recorded as closed by E1 to E5; HQL-0033 is accepted and implemented.
- V2 done: HQL-0034's placeholders are stated: `std/validators.hql`, the
  first written module of the standard library, loaded on `import
  validators` before the module roots, with `Email` and `NonEmpty`; several
  types by one `impl` each, named in a rule; and the identity `Contract.f`.
  HQL-0034 is accepted and implemented.
- B1 done: HQL-0035 states the `hql.toml` form; `Configured.fields` holds
  one `FieldMapping` per field, its properties `None` where unwritten, for
  the resolver to merge; refusals pinned in `tests/sources.rs`.
- B2 done: `src/boundary.rs` holds the resolver HQL-0035's new section on
  it states: the format defaults and decoder tables, field-level recursive
  override with an origin per property, and the five boundary errors,
  pinned by unit tests. It takes the element type's fields and, where the
  format knows them, the external fields, so B3 supplies both.
- B3 done: `source("id")` expecting a `Seq` of a record reads through
  `configured_source`: the boundary renames each document's external
  entries to the HQL fields, SQLite selects only the readable columns and
  checks them, a CSV header is checked, and `tap("id")` remaps in its decode
  traversal. A configured `element` is checked against the boundary at
  check time. Cases in `examples/sources/`, the SQLite column selection in
  `tests/sources.rs`.
- B4 done: HQL-0035's section on nullability at the boundary states the
  rules; `remapped` applies each field's `absent`, refusing an unlisted
  `null` in an optional field and reading unlisted empty text as text.
