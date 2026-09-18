# 0009 — A function type, and functions as values

Status: backlog

HQL has lambdas but no functions. `c => c.title` parses into `Kind::Lambda` and
works in an argument slot, and that is the whole of it: both the checker and the
evaluator reject a lambda anywhere else with *"a lambda may only be written as
an argument"*. There is no way to bind one to a name, no implemented way to declare one, no
type to annotate a parameter with, and no `Value` variant to carry one. Named
callables — `map`, `filter`, `sort` — are Rust steps in the extension registry,
reachable only by writing Rust.

This issue makes a function an ordinary value of an ordinary type: written in
the grammar, carried by the runtime, and checked by the type system.

## The type system is the load-bearing part

Three properties of the current system have to change before a function type can
be declared at all.

**Variance has no contravariant case.** `Variance` offers `Covariant` and
`Invariant` only, and the ancestry walk honours exactly those. A function narrows
another when its result narrows and its parameters *widen*, so the parameter
positions need a third case, and the substitution check needs to honour it.

**A declaration's parameter list is fixed.** `TypeDefinition` carries a `Vec` of
named parameters, one shape per constructor, but functions come in every arity.
Two ways out, and the issue should pick one before any code: a variadic
declaration whose last parameters are the argument positions, or a family of
fixed-arity constructors with the arity in the name. The first keeps one name in
diagnostics and complicates the checker; the second keeps the checker as it is
and puts arity into every error message.

**No carrier.** `Value` has no function variant. Since the evaluator's scope is a
`HashMap<String, Value>`, a function value has to be a closure — parameters,
body, and the scope captured where it was written — or the language has functions
that silently lose their free names.

## Scope

- Add the contravariant case and honour it where a reference is checked against
  a target.
- Declare the function constructor among the built-ins, with parameter positions
  contravariant and the result covariant.
- Add the closure carrier to `Value`, and report its type from `type_of`.
- Lift the argument-position restriction in both the checker and the evaluator: a
  lambda becomes an expression that yields a function value, and `f = c =>
  c.title` checks and evaluates.
- Implement the declaration form the design already fixes:
  `fn name<T>(x: Seq<T>) : T?`, with the function type spelled `T -> U`, as
  [generic brackets](../models/behavior/bind-vs-apply-in-generic-types.md)
  decides and `std/collections.hql` already writes in its operation comments.
  The spelling is not open; only its absence from
  [the expression model](../models/behavior/expressions.md) is, since that file
  records what the parser accepts.
- Resolve a call whose callee is a bound function. `Kind::Call` currently
  demands an identifier and resolves it through the step registry, failing with
  *"expected a named function"* for everything else — the fallback is a scope
  lookup.
- Let a step take a named function where it takes a lambda. `Contract::lambda`
  and `apply_lambda` are handed an `&Arg` and match on `Kind::Lambda`
  themselves, including counting parameters to detect a comparison; they should
  receive a function value and ask it its arity.

## Examples come first

Per the corpus convention, the examples are where the design is settled, so they
are written before the implementation — a named function declared and called, a
lambda bound to a name and passed to `map`, a function taken as a parameter, and
an `invalid` case pinning the diagnostic for an arity or parameter-type
mismatch.

One obstacle: `tests/corpus.rs` asserts `implementation: implemented` for every
case it loads, so an example cannot sit in the tree ahead of its feature. Either
land each example with the code that makes it pass, or give the harness a status
it skips. The second is worth doing on its own — the corpus is the design
record, and it currently cannot hold a design that is not yet built.

## Done when

- A function type is declared, prints readably, and narrows correctly in both
  directions: a function accepting a wider parameter and returning a narrower
  result is accepted where the narrower one is expected, and the reverse is
  rejected.
- A named function can be declared, bound, called, passed to a step, and
  returned from another function.
- `f = c => c.title` checks and evaluates; the message about lambdas being
  argument-only is gone from both layers.
- Examples under `examples/` cover each of the above and pass through
  `tests/corpus.rs` with declared types and results, including at least one
  `invalid` case.
- Unit tests cover the contravariant substitution rule directly, independently
  of any program that exercises it.
