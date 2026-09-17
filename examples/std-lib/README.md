# Standard library design cases

These cases work out the parts of HQL that the query-shaped examples take for
granted: what a trait is, how `Card` gets more than one implementation, how a
`Data` tree earns a stronger type, and how optional values and functions are
written. They deliberately contain no pipelines. Pipe composition is
fundamental to HQL, but every decision here has to stand on its own first.

Everything below is design input. The implemented slice is still Int/Float/Bool
literals and same-type addition, and no `trait`, `impl`, `fn`, `match` or
`refine` form is accepted by today's parser. Read the status field on each case.

## Traits replace interfaces

A trait states a **required surface**. It is not a supertype, and no value has
the type `Card`:

- [trait-card.hql](trait-card.hql) declares the surface — a header, a body, a
  name and uplinks. Its counterpart [card-narrowing-alternative.hql](card-narrowing-alternative.hql)
  keeps the reading currently written in the wiki cards, where `Card <: Doc` is
  a narrowing with one type and no dispatch.
- [impl-data-card.hql](impl-data-card.hql) is the most general implementor. A
  JSON or YAML document is a header with no prose, so `body` is empty `Content`
  and `uplinks` is empty. Anything that can be a `Data` tree can be a card.
- [impl-md-card.hql](impl-md-card.hql) adds a real body and sections. Plain
  Markdown has no wikilink dialect, so asking it for uplinks is a well-formed
  question with an empty answer.
- [impl-hmd-card.hql](impl-hmd-card.hql) is the richest: a body whose links are
  references. The format condition lives here, in an implementation, and never
  in card-ness itself.

That split is the point. `Card` used to be a document narrowed by conditions,
which forced one type to cover a YAML file and an HMD vault card at once. As a
trait, card-ness becomes a surface that three unrelated representations can
provide, and `HmdCard` names a concrete implementation rather than an alias for
the type — a different role from the `HmdCard` name that was dropped earlier.

Dispatch is then ordinary: [trait-dispatch.hql](trait-dispatch.hql) writes one
function over any implementor, and [trait-bound-generic.hql](trait-bound-generic.hql)
asks whether that argument form is sugar for a named bound.
[trait-default-member.hql](trait-default-member.hql) keeps implementations cheap,
[trait-coherence.hql](trait-coherence.hql) is the negative case two extensions
would hit, and [impl-spelling-alternative.hql](impl-spelling-alternative.hql)
keeps a non-Rust spelling alive, because the core cards rule out importing Rust
syntax wholesale.

## Strict typing, not duck typing

The shape of a value is either declared or proved. It is never assumed:

- [record-type.hql](record-type.hql) declares a shape, so access is checked
  against the declaration. [record-strictness.hql](record-strictness.hql)
  rejects an incomplete literal at construction rather than at first read.
- [refinement-progressive.hql](refinement-progressive.hql) shows the other
  direction: one unchanged `Data` tree, with HMD's reserved keys known and the
  neighbouring authored keys still `Data`.
- [refinement-structural.hql](refinement-structural.hql) is the rigorous form of
  duck typing. The tree never declares itself a `PostgresConfig`; it is usable as
  one because the required shape was established, and the access is checked
  inside that branch only.
- [refinement-failure.hql](refinement-failure.hql) makes narrowing fallible, and
  [newtype.hql](newtype.hql) with [newtype-mismatch.hql](newtype-mismatch.hql)
  ask for nominal distinctness where structure alone would let two unrelated
  strings mix.

## Absence and functions

- [option-declaration.hql](option-declaration.hql) declares `Option` as an
  ordinary sum type: there is no null, and absence is a value.
  [sum-type-alternative.hql](sum-type-alternative.hql) avoids spelling variants
  with the bar that pipes already claim.
- [option-match.hql](option-match.hql) relies on exhaustiveness — the missing
  `None` arm is a type error, which is what replaces the null check — and
  [option-map.hql](option-map.hql) covers the uses that do not deserve a match.
- [function-declaration.hql](function-declaration.hql) weighs `fn` against the
  arrow-typed binding already in [types/function.hql](../types/function.hql),
  and [higher-order.hql](higher-order.hql) shows what needs no trait at all.

## Open decisions these cases raise

1. Is `Card` a trait with implementors or a narrowed `Doc`? The two readings
   cannot both survive; the wiki cards still state the second.
2. Does a trait require fields as well as functions, and does a default member
   see overridden members?
3. What governs two implementations of one trait for one type across extensions?
4. Which carrier does a failed refinement return, and does `as` ever skip the
   check?
5. Do variants use a bar, given that `|` is the pipe operator?
6. Is `fn` curried, and is `impl Card` in argument position sugar for a bound?

Generic types are written with angle brackets here, following the wiki cards;
older cases elsewhere in the corpus keep the square-bracket spelling. That
divergence is unresolved and is not a claim that either form is implemented.
