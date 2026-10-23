# 0021 — `of_type` replaces `typed`

Status: done (feature 4 of [0014](0014-next-feature-bundle.md))
Decisions: [HQL-0016](../proposals/HQL-0016/README.md), its sections "`is(U)`
tests a type", "`filter` narrows, and `of_type(U)` is its shorthand" and
"`typed` is removed"
Related: [0018](0018-operator-synonymy-audit.md),
[HQL-0015](../proposals/HQL-0015/README.md)

`typed(U)` is removed with no alias and no deprecation period. The selection
it made is `of_type(U)`, the shorthand for `filter(x => x | is(U))`. The
proposals and issue 0018 already read `of_type`; the code, the tests and the
documents that describe today's behaviour still read `typed`.

## Work

1. **`is(U)`.** Add the step `is<T, U>(x: T, type: Type<U>) : Bool`, with the
   refusal `typed` gives today when no `T` can be a `U` (now: always false)
   and a warning when `T` is already a `U` (always true).
2. **The narrowing `filter`.** When the predicate is `x => x | is(U)` or
   `x => is(x, U)` on the lambda's own parameter, the checker types the result
   `F<U>`; any other predicate keeps the element type. `filter` over an
   `Option<T>` narrows the same way.
3. **`of_type(U)`.** Define it by its longhand. Until `:=`
   ([HQL-0015](../proposals/HQL-0015/README.md)) and its type parameters are
   implemented, register it as a step in `src/extensions/collections.rs` whose
   check and evaluation are those of the narrowing `filter`.
4. **Drop `typed`.** Remove the step. `typed` becomes an unknown operation
   whose diagnostic names `of_type`.
5. **Tests.** `tests/vault.rs` and `tests/extension_contracts.rs` call
   `of_type`; add the cases in HQL-0016's Test Plan for `is`, the narrowing
   `filter` and the `typed` diagnostic.
6. **Documents that describe the implementation.** The
   [type interpretation](../wiki/hql/type-interpretation.hmd) card already
   states `is` and `of_type`; its closing status paragraph drops the former
   name once the step is renamed. Issue
   [0015](0015-jq-parity-analysis.md) and issue
   [0016](0016-sql-parity-analysis.md) read `of_type(…)` where they say
   `typed(as = …)`, and the
   [operation signatures](../models/behavior/operation-signatures.md) inventory
   lists `is` and `of_type`.
7. **Record interpretation is `of_type` with the structural test.** 0016's
   proposed operation, which reads a `Data` value as a declared record,
   extended `typed`. Under the type interpretation card it is `of_type` over
   `Data`, which passes a tree whose shape the record declares; it waits on the
   structural test, not on a name of its own.

## Done when

- `typed` appears in no `.rs`, `.hql`, `.hmd` or `.md` file outside the
  `typed` diagnostic and HQL-0016.
- `hql builtins` lists `is` and `of_type` and not `typed`.
- `cards | of_type(RelationCard)` checks as `Set<RelationCard>`, and
  `cards | of_type(RelationCard) | count` gives the counts `typed` gave.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.

## Outcome

Done. Two items are left where they belong: the always-true warning on `is`
waits for check-time warnings, which the checker does not yet emit; `filter`
over a single `Option<T>` waits for the option operations of `std/core/mod.hql`.
Record interpretation over `Data` is the structural test, feature 6 of 0014.
Issue and accepted-proposal records that describe the former name as history
keep it.
