# 0024 — Partial application

Status: backlog
Source: [0010](0010-yield-and-bind.md), which no longer carries `bind`

Fixing some arguments of a function and getting back a function of the rest
is written today as a lambda that captures a binding:

```hql
add = (a: Int, b: Int) => a + b
k = 10
[1, 2] | map(n => add(k, n))      // [11, 12]
```

That works and stays. This issue decides whether HQL gets a shorter form, and
if so, which one. The [queries model](../models/behavior/queries.md) places the
question in the function design, not in query sources: section "Partial
application is not part of the source model".

## Work

1. Collect the cases in `examples/` where a lambda only fixes arguments, and
   decide from them whether a shorter form earns its place.
2. If it does, choose the form: a library operation (`partial(f, a)`), call
   syntax with a hole, or something else. The name `bind` is avoided, because
   binding a type parameter is a term of the
   [generic brackets](../models/behavior/bind-vs-apply-in-generic-types.md)
   model.
3. Write the result where the function design lives: the function type card,
   the queries model section, and the operation signature, if one is added.
4. Implement it, with examples and `invalid` cases.

A library operation over any arity needs generic functions,
[0022](0022-generic-functions.md).

## Done when

- The decision is stated in the function design. "No shorter form" is a valid
  outcome, and closes this issue once written.
- If a form is added: it type-checks against arity and parameter types, a
  mismatch is refused naming the parameter, and examples cover it through
  `tests/corpus.rs`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
