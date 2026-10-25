# Error handling: checking is the boundary

Every program here is wrong on purpose, and each one says where. Together they
pin [HQL-0044](../../doc/proposals/HQL-0044/README.md): a program is parsed,
then checked as a whole, and only a checked program executes. They also pin
the error model of [HQL-0045](../../doc/proposals/HQL-0045/README.md): a
failure is data or it is a diagnostic, and every diagnostic names its
`phase`, its `kind`, its `severity` and its `disposition`, and travels on the
run's one diagnostic channel. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/error-handling/types/never-called-function.hql
target/debug/hql --report collect run examples/error-handling/boundary/effect-then-three-errors.hql
target/debug/hql run examples/error-handling/boundary/effect-then-overflow.hql
```

Each invalid case's header names the diagnostic it produces by its axes,
`phase:`, `kind:` and `severity:`, and the harness compares all three. A
diagnostic's text form is headed the same way: `check error[type-mismatch]:`.

| Phase | Heading | Kinds | What it means |
| --- | --- | --- | --- |
| parse | `parse failure[…]` | `unexpected-token`, `numeric-overflow` | no syntax tree exists, so nothing is resolved or typed |
| check | `check error[…]` | `unknown-name`, `unknown-field`, `unknown-step` | a name, field, step, type, module or trait does not resolve |
| check | `check error[…]` | `type-mismatch`, `non-exhaustive-match`, `unsatisfied-bound`, `missing-implementation`, `duplicate-implementation` | a type, a match, a bound or a trait contract does not hold |
| execute | `execute failure[…]`, `execute error[…]` | `numeric-overflow`, `division-by-zero`, `io-failure`, `malformed-data`, `contract-violation` | the program checked, and a value made the run fail |

The severity is the producer's, not the phase's: a contract read `x | T` is
an `execute error`, a division by zero an `execute failure`. The cases under
`execute` record `expected-type:`: `hql check` succeeds on them and prints
that type. The harness asserts it.

## The boundary itself

Each of these opens with an effect, `["before"] | target(stdout, Lines)`,
which writes `before` the moment it runs. Whether `before` is printed shows
which side of the boundary the error is on.

| Case | Error | `before` printed |
| --- | --- | --- |
| [syntax failure after the effect](boundary/effect-then-syntax-failure.hql) | `parse failure[unexpected-token]` | no |
| [type error after the effect](boundary/effect-then-type-error.hql) | `check error[type-mismatch]` | no |
| [three check errors after the effect](boundary/effect-then-three-errors.hql) | `unknown-name`, then two `type-mismatch` | no, under `strict` and `collect` |
| [overflow after the effect](boundary/effect-then-overflow.hql) | `execute failure[numeric-overflow]` | yes |

The sessions pin what `hql` prints for them, standard error included:

| Session | What it shows |
| --- | --- |
| [syntax failure runs nothing](boundary/sessions/syntax-failure-runs-nothing.session) | the diagnostic alone, exit `1` |
| [type error runs nothing](boundary/sessions/type-error-runs-nothing.session) | `hql check` and `hql run` print the same diagnostic; `run` writes no `before` |
| [collect lists and runs nothing](boundary/sessions/collect-lists-and-runs-nothing.session) | three diagnostics and a count; no `before` |
| [overflow after the effect](boundary/sessions/overflow-after-the-effect.session) | `before`, then the evaluation failure |
| [envelope](boundary/sessions/envelope-type-before-value.session) | a check error carries `"type": null`; an execute failure carries `"type": "Int"` and `"value": null`; the channel's contents are under `diagnostics` |
| [module contract](boundary/sessions/module-contract.session) | a private helper and an undeclared function, under the module root |
| [REPL](boundary/sessions/repl-rejected-input.session) | a rejected input rebinds nothing: `x` stays `1`; each line typed is its own origin, `<repl:N>` |
| [document](documents/sessions/render-checks-every-cell.session) | [every cell checks](documents/every-cell-checks.hmd) and renders; [a bad last cell](documents/last-cell-type-error.hmd) stops the build before the first |
| [cell origin](documents/sessions/cell-origin.session) | a cell's diagnostic names the cell, `path#2`, as its origin; the text form adds `= cell:` |

## Diagnostics: the error model

| Case | Diagnostic | What it shows |
| --- | --- | --- |
| [warning, then failure](diagnostics/warning-then-failure.hql) | `check warning[naming-convention]`, then `execute failure[division-by-zero]` | one channel carries both, in the order written |
| [collect past an element](diagnostics/collect-past-an-element.hql) | `execute failure[division-by-zero]` | under `collect` `map` steps past the element, `disposition: recoverable` |
| [collect stops at an overflow](diagnostics/collect-stops-at-overflow.hql) | `execute failure[numeric-overflow]` | `fatal` under both modes |
| [errs are data](diagnostics/errs-are-data.hql) | none | `validate` keeps the failure as an `Invalid` value; the channel stays empty |
| [read back](diagnostics/read-back.hql) | none | a later run reads delivered diagnostics, `run.jsonl`, as `Diagnostic` records |
| [as text](diagnostics/as-text.hql) | none | `format(DiagnosticText)` writes delivered diagnostics as the report a run prints |
| [call context](diagnostics/call-context.hql) | `execute failure[division-by-zero]` | the `call` frames that reached the failure, and the `bindings` where it failed |
| [card provenance](diagnostics/card-provenance.hql) | `execute failure[division-by-zero]` | a failure over a card's value names the card's document as its `source`; runs against `diagnostics/vault` |

`diagnostics/vault/` is a vault, not a set of cases: one card whose header
parses, with `pages: 0`, and one whose header does not. `diagnostics/run.jsonl`
is what `--diagnostics-format json` wrote for `warning-then-failure.hql`.

| Session | What it shows |
| --- | --- |
| [four axes](diagnostics/sessions/four-axes.session) | a parse, a check and two execute diagnostics as JSON, each with its `phase`, `kind`, `severity` and `disposition`; the contract read carries `expected` and `violations`, an element failure its `element` |
| [channel order](diagnostics/sessions/channel-order.session) | the check warning, then the execute failure, in JSON and as text |
| [collect steps past an element](diagnostics/sessions/collect-steps-past-an-element.session) | `strict` ends the run; `collect` gives `[1, 2]` and the same diagnostic, recoverable; an overflow is fatal under both |
| [help and labels](diagnostics/sessions/help-and-labels.session) | a nearest name is `help`, a second place a `note`, and neither is in the message |
| [the vault skips a document](diagnostics/sessions/vault-skips-a-document.session) | `load warning[skipped-document]`, the rest of the vault read, exit `0` |
| [collect through steps](diagnostics/sessions/collect-through-steps.session) | `filter` and `flat_map` step past a failing element under `collect`, each naming itself |
| [call context](diagnostics/sessions/call-context.session) | `= bindings:`, `= trace:` and `= source:` under the caret |

## Parse

| Case | Diagnostic |
| --- | --- |
| [unclosed group](parse/unclosed-group.hql) | `parse failure[unexpected-token]` |
| [empty match](parse/empty-match.hql) | `parse failure[unexpected-token]` |
| [chained comparison](parse/chained-comparison.hql) | `parse failure[unexpected-token]` |
| [import of a file](parse/import-of-a-file.hql) | `parse failure[unexpected-token]` |
| [duplicate parameter](parse/duplicate-parameter.hql) | `parse failure[unexpected-token]` |
| [integer literal out of range](parse/integer-literal-out-of-range.hql) | `numeric-overflow` of the parse phase; compare the overflow under evaluation |

## Names

| Case | Diagnostic |
| --- | --- |
| [unbound name](names/unbound-name.hql) | `check error[unknown-name]` |
| [unknown field](names/unknown-field.hql) | `check error[unknown-field]` |
| [field of a primitive, in a lambda](names/field-of-a-primitive.hql) | `check error[unknown-field]` |
| [unknown type](names/unknown-type.hql) | `check error[unknown-name]` |
| [unknown module](names/unknown-module.hql) | `check error[unknown-name]` |
| [unknown receiver function](names/unknown-receiver-function.hql) | `check error[unknown-name]`, with the nearest name |
| [unknown trait](names/unknown-trait.hql) | `check error[unknown-name]` |
| [constructor with an unknown field](names/constructor-unknown-field.hql) | `check error[unknown-field]` |
| [misspelled step](names/misspelled-step.hql) | `check error[unknown-step]`, with the nearest step |
| [private helper](names/private-helper.hql) | `check error[unknown-name]`; needs `modules: error-handling/lib` |
| [module has no such function](names/module-has-no-function.hql) | `check error[unknown-name]`; needs `modules: error-handling/lib` |

## Types

| Case | Diagnostic |
| --- | --- |
| [String plus Int](types/string-plus-int.hql) | `check error[type-mismatch]` |
| [declared binding](types/declared-binding.hql) | `check error[type-mismatch]` |
| [argument type](types/argument-type.hql) | `check error[type-mismatch]` |
| [argument count](types/argument-count.hql) | `check error[type-mismatch]` |
| [declared result](types/declared-result.hql) | `check error[type-mismatch]` |
| [an Option is not its value](types/option-is-not-its-value.hql) | `check error[type-mismatch]` |
| [a Result is not its value](types/result-is-not-its-value.hql) | `check error[type-mismatch]` |
| [match misses a union member](types/match-misses-a-member.hql) | `check error[non-exhaustive-match]`, though the value passed is covered |
| [match misses none](types/match-misses-none.hql) | `check error[non-exhaustive-match]`, though the value is `some(1)` |
| [predicate over an empty list](types/predicate-over-empty-list.hql) | `check error[type-mismatch]`, with no element to apply it to |
| [never-called function](types/never-called-function.hql) | `check error[type-mismatch]`, though nothing calls it |
| [lambda arity](types/lambda-arity.hql) | `check error[type-mismatch]` |
| [record field type](types/record-field-type.hql) | `check error[type-mismatch]` |
| [type argument count](types/type-argument-count.hql) | `check error[type-mismatch]` |
| [connective needs Bool](types/connective-needs-bool.hql) | `check error[type-mismatch]` |
| [index type](types/index-type.hql) | `check error[type-mismatch]` |
| [condition not Bool](types/condition-not-bool.hql) | `check error[type-mismatch]` |
| [branches disagree](types/branches-disagree.hql) | `check error[type-mismatch]`, though the call takes the `Int` branch |
| [no renderer](types/no-renderer.hql) | `check error[type-mismatch]` |

## Traits: capability contracts

| Case | Diagnostic |
| --- | --- |
| [lacks a required function](traits/lacks-required-function.hql) | `check error[missing-implementation]` at the `impl` |
| [second implementation](traits/second-implementation.hql) | `check error[duplicate-implementation]`: coherence |
| [wrong result type](traits/wrong-result-type.hql) | `check error[type-mismatch]` |
| [wrong parameter count](traits/wrong-parameter-count.hql) | `check error[type-mismatch]` |
| [function not in the trait](traits/function-not-in-trait.hql) | `check error[unknown-name]` |
| [body checked in the impl](traits/body-checked-in-impl.hql) | `check error[type-mismatch]` |
| [bound not satisfied](traits/bound-not-satisfied.hql) | `check error[unsatisfied-bound]` at the call |
| [extended trait not implemented](traits/extended-trait-not-implemented.hql) | `check error[unsatisfied-bound]` |
| [a trait is not a type](traits/trait-is-not-a-type.hql) | `check error[unknown-name]` |
| [impl for an unknown type](traits/unknown-impl-type.hql) | `check error[unknown-name]` |
| [no implementation for the receiver](traits/no-implementation-for-receiver.hql) | `check error[unknown-name]` |
| [unbounded type parameter](traits/unbounded-parameter.hql) | `check error[unknown-name]` |
| [free call of a trait function](traits/free-call-of-trait-function.hql) | `check error[unknown-name]` |
| [operator without Add](traits/operator-without-add.hql) | `check error[type-mismatch]` |
| [sort without Ord](traits/sort-without-ord.hql) | `check error[unsatisfied-bound]` |
| [Add with the wrong parameter type](traits/add-wrong-parameter-type.hql) | `check error[type-mismatch]` |
| [Ord returning a number](traits/ord-wrong-result.hql) | `check error[type-mismatch]` |
| [Eq with another type](traits/eq-with-another-type.hql) | `check error[type-mismatch]` |

## Execution: past the boundary

Each of these checks; the failure depends on a value.

| Case | Checked type | Diagnostic |
| --- | --- | --- |
| [overflow](evaluation/overflow.hql) | `Int` | `execute failure[numeric-overflow]` |
| [division by zero](evaluation/division-by-zero.hql) | `List<Int>` | `execute failure[division-by-zero]` |
| [missing file](evaluation/missing-file.hql) | `Int` | `execute failure[io-failure]` |
| [fail-fast read](evaluation/fail-fast-read.hql) | `Int` | `execute error[contract-violation]` |
| [malformed JSON](evaluation/malformed-json.hql) | `Data` | `execute failure[malformed-data]` |
| [handled read](evaluation/handled-read.hql) | `List<String>` | none: the failure is an `err` value, exit `0` |

`lib/` is a module root, not a set of cases: `shapes.geometry` with one
public function and one private helper.
