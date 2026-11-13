# 0075 — Validation: the rules of `Violation` and `Invalid`, quality thresholds, validation events

Status: backlog
Related: [0043](archive/0043-contract-validation.md), whose remainder this
is; [HQL-0032](../proposals/HQL-0032/README.md),
[HQL-0033](../proposals/HQL-0033/README.md),
[issue 0045](archive/0045-errors-validators-boundaries.md)

Contract rules, structured validation results and the stream policies are
built ([0043](archive/0043-contract-validation.md)): `x | T` enforces a
contract's rules, `x | validate(T)` gives `Result<T, Invalid>` over a value,
a collection or a stream, `oks()` and `errs()` split the results, and a dead
letter is `fork { accepted = oks(), rejected = errs() }` with `errs()`
written to a sink. Three parts of 0043's scope are not built.

## Work

1. **`Violation`'s and `Invalid`'s own rules.** Both are declared as
   `DataContract`s in `lib/std/core/mod.hql` with fields and defaults and no
   rules. State the rules a value of each must hold (for example, that
   `severity` is one of the severities a diagnostic has, and that an
   `Invalid` holds at least one violation), so that reading a stored
   `Invalid` back with `| Invalid` checks them.
2. **Error aggregation and quality thresholds over a stream.** HQL-0032's
   "Stream validation" and "Dataset-level constraints" sections name error
   aggregation, threshold-based failure, duplicate thresholds and null-rate
   thresholds. Decide which are pipelines over `Result` values, as the
   stream policies turned out to be, and which need a step of their own;
   build the ones that need a step.
3. **Validation events and hooks.** HQL-0032's "Validation events" section
   names `.validation.started`, `.validation.succeeded`, `.validation.failed`
   and `.validation.warning`, each carrying a structured result, and HQL-0030
   names contract hooks. Both wait on an event model; this item states what
   validation emits once there is one.

## Done when

- Reading a value as `Violation` or `Invalid` enforces their rules, with a
  case in [the structural examples](../../examples/structural/README.md).
- A stream read through a contract can fail when the share of invalid
  elements passes a stated threshold, and the program that says so is in
  the Learn page
  [Validation as a value](../public/learn/11-advanced-data-typing/validation-as-a-value.md).
- Validation events are specified against the event model, or this item is
  moved to the issue that builds the event model.
