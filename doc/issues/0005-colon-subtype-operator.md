# 0005 — Replace `<:` with `:`

Status: done
Branch: `feat/colon-subtype`

HQL is to declare a supertype with `:`:

```hql
type X : Y
type X : {Y, Z}
```

`type X : Y` declares `Y` as a supertype of `X`. `type X : {Y, Z}` declares a
**set** of supertypes — literally a set holding two of them, not a union and not
an intersection operator spelled with braces.

## Why `:` rather than `<:`

`:` is the conventional type-theoretic notation for a judgment, `x : T`, meaning
that `x` has type `T`. Set-theoretic membership is conventionally written
`x ∈ A`, so `:` is not overloaded by being used here.

The explicit `type` keyword already makes the declaration context clear:

```hql
type Card : Doc
type HmdCard : {Card, Doc}
```

A line that opens with `type` declares a supertype; one that does not is a
typing judgment about a value. `<:` therefore added no information the keyword
did not already carry, and one notation serves both type declarations and
typing contexts.

## Scope

- `Token::Subtype` and its scanning branch leave the lexer; `<:` stops being a
  token, so the sequence lexes as `<` followed by `:` and is refused.
- The supertype position and the `where` bound both read `Token::Colon` —
  `type SortedMap<K, V> : OrderedMap<K, V> where K : Orderable`.
- `std/`, the examples, the tests, and the grammar in
  [expressions](../models/behavior/expressions.md) take the new spelling.
- [type-system](../wiki/hql/type-system.hmd) is rewritten to state the
  one-symbol rationale rather than to contrast two symbols.
- **Not a semantic change.** What a declaration means, what a supertype set
  means, and what the checker refuses are all unchanged.

## Done when

- `cargo test` passes and `hql check std/knowledge.hql` prints `Unit`.
- A test pins `<:` as a syntax error, so the old operator cannot return quietly.
- No `<:` survives as accepted syntax in `src/`, `std/`, `tests/`, `examples/`,
  or `doc/`. The rejection tests and migration explanation retain it, as does
  the [design conversation](../conversations/sets-n-subtypes.md), whose original
  wording is preserved with a note linking to the current syntax.

## Validation

- `cargo test --offline --locked --no-fail-fast`: 136 tests pass.
- `cargo clippy --offline --locked --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `hql check std/knowledge.hql`: `Unit`.

Colon declarations with a record body, supertype sets, and `where` bounds are
covered by tests. The declaration checker and supertype normalization are
unchanged.

The working prompt is [0005-colon-subtype-operator.prompt.md](0005-colon-subtype-operator.prompt.md).
