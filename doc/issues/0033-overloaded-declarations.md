# 0033 — Overloaded declarations: every standard operation declared in HQL

Status: done
Related: [0030](0030-hql-only-signature-authority.md),
[0018](0018-operator-synonymy-audit.md),
[the extensions card](../wiki/hql/extensions.hmd),
[operation signatures](../models/behavior/operation-signatures.md)

A name declared more than once in a `lib/std/` module is one operation with
several signatures, and a call takes the first declaration its input and its
arguments fit. `S : Seq<T>` holds the input at its own type,
`F<_> : Iterable<_>` rebuilds its constructor as `F<U>`, and `Type<U>` takes a
type name. The core's collection operations, `seq`'s `reverse`, `distinct`,
`take_while`, `drop_while`, `min_by`, `max_by`, `with`, `without`,
`intersection` and `difference`, `math`'s `abs`, `pow` and `sign`, and `map`
and `filter` over one `Option` and `map` over a `Result` take their signatures
from their declarations. So do `format` and its shorthands, `fmt`, the
narrowing `filter`, and the positional arguments of `count`, `contains`,
`get` and `pow`.

## Scope

- `format` in `lib/std/format/mod.hql` is declared: `format<T, F : Format>(x: T, as:
  Type<F>, …)`, whose result over a stream and over a finite input are two
  overloads, as `collect`'s are. `Output::Finite` is then unused and removed.
- The narrowing `filter(x => x | is(U))`, which returns `F<U>`, is stated by
  a declaration rather than by `Checker::step`.
- `collect(for = …)`, a reserved argument, is refused by its declaration
  rather than by `collections::reserved`.
- A variadic parameter takes a value of any type in each position, so that
  `fmt` is declared: `fmt("{}", some(1))` is refused by `...Data`, and
  `...T` binds one type for every argument.
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

- No core, `seq`, `math`, `string` or `format` operation has a type written in Rust,
  and `Checker::step` names no operation.
- `Output` holds only `Type` and `Computed`.
- Each item above has a test.

## Decisions taken while implementing

- **Positional and named only.** No card named a syntax, so an external
  declaration writes `/` after the parameters a call supplies by position
  only, and `*` before those it supplies by name only:
  `fn count<T>(xs: Collection<T>, value: T, /) : Int`,
  `fn format<T, F : Format>(x: T, as: Type<F>, *, limit: Int = 0, …)`. A
  function with a body takes every argument by position already, so the
  markers belong to an external declaration, as a default does.
- **Overload selection.** A call takes the first declaration whose input,
  arity and value arguments' types it fits; a function and a type name are
  checked against the declaration chosen, not read to choose it. A call that
  fits a declaration by its arity and none by its types is refused naming
  every declaration of that arity when another one takes its arguments
  (`2 | pow(2.0)`); otherwise the first declaration it fits is checked, so
  the refusal names the argument that is wrong (`[1] | take(true)`).
- **Shadowing.** Binding refuses a declaration when an earlier one of the
  name takes the same arguments, may omit as many, and accepts every input
  and value argument it does. Where the patterns alone cannot tell, the
  declaration is kept.
- **Absence lifting.** A step whose chosen declaration does not take a
  collection is lifted over an optional input: `none | abs` is an
  `Option<Int>` and absent, `some(-2) | abs` is present, and a result that
  is already an `Option` is not wrapped again. A step over a collection
  still takes absence as the empty collection, `none | json` is `"[]"`.
- **`collect(for = …)`** is refused as an argument the declaration does not
  name: `` `collect` has no argument `for` ``.
- **`format`** is three declarations per name, over a `Collection<T>`, a
  `Stream<T>` and any `T`, since a sequence is a stream that ends and the
  first form a call fits is taken. The eleven shorthands are declared the
  same way.
- **The narrowing `filter`** is declared with a predicate of type
  `T -> Is<U>`: such a parameter takes only a lambda written
  `x => x | is(U)` and binds `U` to the type it tests. `Is<T>` is declared in
  `lib/std/core/mod.hql` beside `Type<T>`.
- **`...Any`** takes a value of any type in each position; `Any` is a
  contract's name for an unconstrained value, not a type a program writes.
