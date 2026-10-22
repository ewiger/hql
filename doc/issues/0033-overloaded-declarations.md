# 0033 — Overloaded declarations: every standard operation declared in HQL

Status: doing
Related: [0030](0030-hql-only-signature-authority.md),
[0018](0018-operator-synonymy-audit.md),
[the extensions card](../wiki/hql/extensions.hmd),
[operation signatures](../models/behavior/operation-signatures.md)

A name declared more than once in a `std/` module is one operation with
several signatures, and a call takes the first declaration its input and its
arguments fit. `S : Seq<T>` holds the input at its own type,
`F<_> : Iterable<_>` rebuilds its constructor as `F<U>`, and `Type<U>` takes a
type name. The core's collection operations, `seq`'s `reverse`, `distinct`,
`take_while`, `drop_while`, `min_by`, `max_by`, `with`, `without`,
`intersection` and `difference`, `math`'s `abs`, `pow` and `sign`, and `map`
and `filter` over one `Option` and `map` over a `Result` take their signatures
from their declarations. What is left below is every signature still written
in Rust or in the checker, and the limits of overload selection.

## Scope

- `format` in `std/format.hql` is declared: `format<T, F : Format>(x: T, as:
  Type<F>, …)`, whose result over a stream and over a finite input are two
  overloads, as `collect`'s are. `Output::Finite` is then unused and removed.
- The narrowing `filter(x => x | is(U))`, which returns `F<U>`, is stated by
  a declaration rather than by `Checker::step`.
- `collect(for = …)`, a reserved argument, is refused by its declaration
  rather than by `collections::reserved`.
- An argument may be declared positional only, as `count(value)`,
  `contains(value)`, `get(key)` and `pow(exponent)` were.
- Overload selection reads argument types where the input and arity do not
  decide: `2 | pow(2.0)` is refused naming both forms of `pow`, not only the
  `Int` one.
- Binding refuses an overload that an earlier one of the same name always
  takes first.
- A constructor parameter takes more than one argument: `M<_, _> : Map<_, _>`.
- `none | abs`, `none | sqrt` and `none | upper` check, and fail at run time
  with "needs a number, not Set<Never>": a step lifted over absence evaluates
  to absence.

## Done when

- No core, `seq`, `math` or `format` operation has a type written in Rust,
  and `Checker::step` names no operation.
- `Output` holds only `Type` and `Computed`.
- Each item above has a test.
