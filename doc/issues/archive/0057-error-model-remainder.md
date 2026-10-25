# 0057 — The error model's remainder: what HQL-0045 left open

Status: done
Related: [HQL-0045](../../proposals/HQL-0045/README.md),
[error-handling examples](../../../examples/error-handling/README.md),
[`src/diagnostics.rs`](../../../src/diagnostics.rs)

HQL-0045 is implemented in part. This issue carries the rest.

## Work

1. `Diagnostic` declared in `lib/std/`, so a diagnostic delivered to a file
   reads back with `source(file(p), JsonLines) | Diagnostic` (unit 6).
2. Execution context: `bindings` in scope at the span, `call` frames of
   `trace`, and `source` provenance for a value read from a card (units 7
   and 8).
3. `collect` through the steps other than `map` whose semantics allow
   stepping past an element: `filter`, `flat_map` and the folds that can
   (unit 9).
4. A rendered cell's diagnostics name the cell as their `origin`, not the
   document.
5. `DiagnosticText` a format a program can name, beside `Json`.
6. The [error-handling card](../../wiki/hql/error-handling.hmd), the
   [reporting model](../../models/behavior/reporting.md) and `cli.md` rewritten
   to the terminology of HQL-0045 (unit 11).

Each lands with a case or a session under `examples/error-handling/` written
first as `proposed`.

## Done

All six landed, each with its case or session: `type Diagnostic` in
`lib/std/core` and `read-back.hql`; `call` frames, `bindings` and a card's
document as `source`, with `call-context.hql` and `card-provenance.hql`;
`collect` through `filter` and `flat_map`, with
`collect-through-steps.session`; a cell's origin `path#cell`, with
`cell-origin.session`; `DiagnosticText` as a format, with `as-text.hql`; and
the card, the reporting model and the command-line model rewritten. A `source`
names the document only, not the line or the key of the value.
