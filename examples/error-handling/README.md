# Error handling: checking is the boundary

Every program here is wrong on purpose, and each one says where. Together they
pin [HQL-0044](../../doc/proposals/HQL-0044/README.md): a program is parsed,
then checked as a whole, and only a checked program executes. From the
repository root:

```sh
cargo build --locked
target/debug/hql run examples/error-handling/types/never-called-function.hql
target/debug/hql --report collect run examples/error-handling/boundary/effect-then-three-errors.hql
target/debug/hql run examples/error-handling/boundary/effect-then-overflow.hql
```

| Phase | Diagnostic | Header | What it means |
| --- | --- | --- | --- |
| parse | `syntax failure` | `SyntaxError`, stage `parse` | no syntax tree exists, so nothing is resolved or typed |
| check | `name error` | `NameError`, stage `typecheck` | a name, field, type, module or trait does not resolve |
| check | `type error` | `TypeMismatch`, stage `typecheck` | a type, a contract or a bound does not hold |
| execute | `evaluation failure` | `RuntimeError` or `IntegerOverflow`, stage `evaluate` | the program checked, and a value made the run fail |

The cases under `evaluate` record `expected-type:`: `hql check` succeeds on
them and prints that type. The harness asserts it.

## The boundary itself

Each of these opens with an effect, `["before"] | target(stdout, Lines)`,
which writes `before` the moment it runs. Whether `before` is printed shows
which side of the boundary the error is on.

| Case | Error | `before` printed |
| --- | --- | --- |
| [syntax failure after the effect](boundary/effect-then-syntax-failure.hql) | syntax failure | no |
| [type error after the effect](boundary/effect-then-type-error.hql) | type error | no |
| [three check errors after the effect](boundary/effect-then-three-errors.hql) | name error, two type errors | no, under `strict` and `collect` |
| [overflow after the effect](boundary/effect-then-overflow.hql) | evaluation failure | yes |

The sessions pin what `hql` prints for them, standard error included:

| Session | What it shows |
| --- | --- |
| [syntax failure runs nothing](boundary/sessions/syntax-failure-runs-nothing.session) | the diagnostic alone, exit `1` |
| [type error runs nothing](boundary/sessions/type-error-runs-nothing.session) | `hql check` and `hql run` print the same diagnostic; `run` writes no `before` |
| [collect lists and runs nothing](boundary/sessions/collect-lists-and-runs-nothing.session) | three diagnostics and a count; no `before` |
| [overflow after the effect](boundary/sessions/overflow-after-the-effect.session) | `before`, then the evaluation failure |
| [envelope](boundary/sessions/envelope-type-before-value.session) | a check error carries `"type": null`; an evaluation failure carries `"type": "Int"` and `"value": null` |
| [module contract](boundary/sessions/module-contract.session) | a private helper and an undeclared function, under the module root |
| [REPL](boundary/sessions/repl-rejected-input.session) | a rejected input rebinds nothing: `x` stays `1` |
| [document](documents/sessions/render-checks-every-cell.session) | [every cell checks](documents/every-cell-checks.hmd) and renders; [a bad last cell](documents/last-cell-type-error.hmd) stops the build before the first |

## Parse

| Case | Diagnostic |
| --- | --- |
| [unclosed group](parse/unclosed-group.hql) | syntax failure |
| [empty match](parse/empty-match.hql) | syntax failure |
| [chained comparison](parse/chained-comparison.hql) | syntax failure |
| [import of a file](parse/import-of-a-file.hql) | syntax failure |
| [duplicate parameter](parse/duplicate-parameter.hql) | syntax failure |
| [integer literal out of range](parse/integer-literal-out-of-range.hql) | syntax failure; compare the overflow under evaluation |

## Names

| Case | Diagnostic |
| --- | --- |
| [unbound name](names/unbound-name.hql) | name error |
| [unknown field](names/unknown-field.hql) | name error |
| [field of a primitive, in a lambda](names/field-of-a-primitive.hql) | name error |
| [unknown type](names/unknown-type.hql) | name error |
| [unknown module](names/unknown-module.hql) | name error |
| [unknown receiver function](names/unknown-receiver-function.hql) | name error, with the nearest name |
| [unknown trait](names/unknown-trait.hql) | name error |
| [constructor with an unknown field](names/constructor-unknown-field.hql) | name error |
| [misspelled step](names/misspelled-step.hql) | name error, with the nearest step |
| [private helper](names/private-helper.hql) | name error; needs `modules: error-handling/lib` |
| [module has no such function](names/module-has-no-function.hql) | name error; needs `modules: error-handling/lib` |

## Types

| Case | Diagnostic |
| --- | --- |
| [String plus Int](types/string-plus-int.hql) | type error |
| [declared binding](types/declared-binding.hql) | type error |
| [argument type](types/argument-type.hql) | type error |
| [argument count](types/argument-count.hql) | type error |
| [declared result](types/declared-result.hql) | type error |
| [an Option is not its value](types/option-is-not-its-value.hql) | type error |
| [a Result is not its value](types/result-is-not-its-value.hql) | type error |
| [match misses a union member](types/match-misses-a-member.hql) | type error, though the value passed is covered |
| [match misses none](types/match-misses-none.hql) | type error, though the value is `some(1)` |
| [predicate over an empty list](types/predicate-over-empty-list.hql) | type error, with no element to apply it to |
| [never-called function](types/never-called-function.hql) | type error, though nothing calls it |
| [lambda arity](types/lambda-arity.hql) | type error |
| [record field type](types/record-field-type.hql) | type error |
| [type argument count](types/type-argument-count.hql) | type error |
| [connective needs Bool](types/connective-needs-bool.hql) | type error |
| [index type](types/index-type.hql) | type error |
| [condition not Bool](types/condition-not-bool.hql) | type error |
| [branches disagree](types/branches-disagree.hql) | type error, though the call takes the `Int` branch |
| [no renderer](types/no-renderer.hql) | type error |

## Traits: capability contracts

| Case | Diagnostic |
| --- | --- |
| [lacks a required function](traits/lacks-required-function.hql) | type error at the `impl` |
| [second implementation](traits/second-implementation.hql) | type error: coherence |
| [wrong result type](traits/wrong-result-type.hql) | type error |
| [wrong parameter count](traits/wrong-parameter-count.hql) | type error |
| [function not in the trait](traits/function-not-in-trait.hql) | name error |
| [body checked in the impl](traits/body-checked-in-impl.hql) | type error |
| [bound not satisfied](traits/bound-not-satisfied.hql) | type error at the call |
| [extended trait not implemented](traits/extended-trait-not-implemented.hql) | type error |
| [a trait is not a type](traits/trait-is-not-a-type.hql) | name error |
| [impl for an unknown type](traits/unknown-impl-type.hql) | name error |
| [no implementation for the receiver](traits/no-implementation-for-receiver.hql) | name error |
| [unbounded type parameter](traits/unbounded-parameter.hql) | name error |
| [free call of a trait function](traits/free-call-of-trait-function.hql) | name error |
| [operator without Add](traits/operator-without-add.hql) | type error |
| [sort without Ord](traits/sort-without-ord.hql) | type error |
| [Add with the wrong parameter type](traits/add-wrong-parameter-type.hql) | type error |
| [Ord returning a number](traits/ord-wrong-result.hql) | type error |
| [Eq with another type](traits/eq-with-another-type.hql) | type error |

## Evaluation: past the boundary

Each of these checks; the failure depends on a value.

| Case | Checked type | Diagnostic |
| --- | --- | --- |
| [overflow](evaluation/overflow.hql) | `Int` | evaluation failure |
| [division by zero](evaluation/division-by-zero.hql) | `List<Int>` | evaluation failure |
| [missing file](evaluation/missing-file.hql) | `Int` | evaluation failure |
| [fail-fast read](evaluation/fail-fast-read.hql) | `Int` | evaluation failure |
| [malformed JSON](evaluation/malformed-json.hql) | `Data` | evaluation failure |
| [handled read](evaluation/handled-read.hql) | `List<String>` | none: the failure is an `err` value, exit `0` |

`lib/` is a module root, not a set of cases: `shapes.geometry` with one
public function and one private helper.
