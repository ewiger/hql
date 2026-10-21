# Part 5: Traits and implementations

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 2, 44 to
  49, and 35 to 41 of the Decision section
- **Depends on**: [modules](01-modules.md) (an implementation belongs to a
  module), [the standard library](03-standard-library.md)
- **Needed by**: [operators](06-operators.md),
  [type system completion](11-type-system-completion.md)

## Scope

A **trait** is a compile-time capability contract: a named set of required
function signatures that a type satisfies through an **implementation**.
Traits exist for generic constraints, operator semantics and algorithms over
unrelated types. They are not introduced for convenience methods, which belong
to [receiver modules](04-receiver-modules.md).

## Rules

### Declaration and constraint

```hql
trait Eq {
    fn eq(self, other: Self) : Bool
}

trait Add {
    fn add(self, other: Self) : Self
}

fn contains<T : Eq>(xs: Seq<T>, value: T) : Bool = ...
```

`T : Eq` reads "`T` satisfies `Eq`". It uses the one relation symbol the type
system already has for a bound. It does not say that values of `T` carry a
runtime `Eq` representation: no inheritance tree, no trait object, no dynamic
dispatch and no existential type is introduced. Traits are compile-time until
a runtime feature separately justifies more.

### Implementation

```hql
impl Add for Int {
    fn add(self, other: Int) : Int = ...
}
```

An implementation associates a trait, a type and the required operations. Its
body may be HQL or native. The draft fixes the semantics and leaves the exact
spelling of `trait` and `impl` to a grammar decision.

### Coherence

For one effective environment there is at most one implementation of a trait
for a concrete type. Two are a compile error naming both; import order never
chooses. An implementation must belong to the module family that defines
either the trait or the type, so a third package cannot change the meaning of
`Add for Int`. Full coherence rules are left by the draft to a dedicated trait
record.

### The core set

`Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Hashable`, `Numeric`, with the
exact set refined separately. `Numeric` is a broad constraint for algorithms
that need numbers; it does not absorb every arithmetic capability, and may be
defined as requiring a subset of the fine-grained ones. Operator traits are
language-owned and live in the prelude environment ([part 6](06-operators.md)).

### Traits are few

`String.trim()` needs no `Trimmable`. A trait is written only when generic
code or language semantics has to quantify over a shared capability.

## Current state

The [type system card](../../../wiki/hql/type-system.hmd) states **there are
no traits**, and gives three mechanisms for what a trait would be asked to do:

| What is said | Today's mechanism |
| --- | --- |
| the value is the thing | ancestry: `type Int : {Scalar, Orderable}` |
| something can be done with the value | an extension registers `render(Graph) -> Html`; the checker consults the registration |
| several types share fields | a partial schema over `Data` |

Bounds are written against **abstract types of law**: `T : Orderable`,
`S : Collection<T>`, `F<_> : Iterable<_>`. An abstract type is declared with
`abstract type Orderable` in [`std/core.hql`](../../../../std/core.hql), has
no runtime constructor, is satisfied by a type naming it in its supertype set,
and its law is enforced behaviorally in Rust. `compare`, `min`, `max`, `sort`
and `SortedMap` keys require `Orderable` ([orderable](../../../wiki/hql/types/orderable.hmd)).

## What changes

- The card's position is reversed in one respect: a capability that generic
  code quantifies over becomes a trait with an `impl`, instead of a
  registration the checker consults. The other two rows stand: ancestry stays
  `:` under `type`, and partial schemas stay with `Data`.
- `abstract type` is not mentioned by the draft, so it stays as declared. The
  two constructs now overlap exactly once: `Orderable` (ancestry, declared in a
  supertype set) and `Ord` (a trait with an implementation) say the same thing
  about `Int`. The design must pick one of:
  - abstract types of law remain the bounds vocabulary for structure
    (`Collection`, `Seq`, `Iterable`, `Endpoint`), and traits cover operations
    a type implements (`Eq`, `Add`, `Ord`), with `Orderable` either kept as the
    ancestry form of `Ord` or retired in its favour;
  - every abstract type used only as a bound becomes a trait, and `abstract
    type` is kept for abstract *collections* that have carriers (`Seq` over
    `List`), which is what the "type of law" argument was written for.
- `Renderable<T>`, which the card leaves as "a registration whose shape is a
  design question", gets its shape from this part: a parameterized trait with
  an implementation per type and format, or stays a registration if traits are
  kept to operators. [Part 11](11-type-system-completion.md) carries the item.
- Inline bounds `<T : Eq>` are the draft's only spelling; the card accepts
  inline and `where`. Both stay unless the grammar decision removes one.
- `Self` inside a trait body is new; the naming card gains it beside `self`.

## Work units

1. The grammar decision for `trait` and `impl`, and `Self`.
2. Trait declarations, implementations, constraint checking at a call and
   the coherence diagnostic.
3. The core trait set in `std/`, with native implementations for `Int`,
   `Float`, `String`, and the `Orderable` versus `Ord` decision applied to
   `std/core.hql`, `compare`, `sort` and `SortedMap`.
4. Updating the type system card's "There are no traits" section and the
   design status card's "Current direction" to the adopted split.
