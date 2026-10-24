# 0052 — Review follow-ups: Decimal in SQLite, quadratic steps, unreviewed code

Status: done
Related: [HQL-0031, Decimal](../proposals/HQL-0031/README.md),
[HQL-0035, schema boundaries](../proposals/HQL-0035/README.md),
[issue 0045](0045-errors-validators-boundaries.md)

Known defects left open when the complete type system branch was merged,
each found by the review of that branch and none blocking it.

## Bugs

1. **A `Decimal` written to SQLite loses its digits.** The SQLite writer
   stores a decimal leaf as `REAL` through `to_float`, unless the column's
   external type is text (`src/extensions/io.rs`, the row writer of the SQLite
   target). `12345678901234567.89` in a `NUMERIC` column reads back as
   `12345678901234568`, which breaks the exactness HQL-0031 states. The
   column's representation belongs to the schema boundary: a decimal field
   SHOULD be written as text, or as an integer of a declared scale, by what
   the boundary declares for it.

## Performance

2. **`merge` of two maps is quadratic.** `merged` (`src/evaluation.rs`)
   finds each right-hand key by a linear scan of the left entries. Two maps
   of 100 000 entries take about 10^10 comparisons. A keyed index of the left
   entries makes it linear.
3. **`distinct` is quadratic.** `eval_distinct` (`src/extensions/seq.rs`)
   checks each value against every one kept so far. A seen-set keyed by the
   value's order key makes it linear and keeps first-occurrence order.

## Cleanup

4. **`decimal.rs` duplicates gcd.** `num_gcd` reimplements Euclid's
   algorithm that `num_integer::Integer::gcd`, through the bigint crate,
   provides. `rounded_quotient` counts digits by formatting each `BigInt` as
   text on every non-terminating division.

## Review

5. **Part of the branch was never read line by line**: most of the checker's
   additions (`src/checker.rs`), `src/parser.rs`, `src/boundary.rs`,
   `src/sources.rs` and `src/extensions/collections.rs`. The test suite and
   corpus cover them; a review pass over each is still due.
