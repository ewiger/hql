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
body may be HQL or native. A native implementation is the header alone:

```hql
impl Add for Int
impl Ord for String
```

The header is the declaration; an extension binds the bodies to it
([part 12](12-contracts-and-binding.md)). That a built-in type implements a
trait is therefore stated in HQL, in [`std/core/mod.hql`](../../../../std/core/mod.hql),
and the runtime implements exactly the headers declared there: binding refuses
a header without a native body and a native body without a header. Only a
module an extension provides declares a bodyless header; a program or a
workspace module writes the functions in braces.

### Coherence

For one effective environment there is at most one implementation of a trait
for a concrete type at one set of the trait's arguments, whether it is native
or HQL; [HQL-0027](../../HQL-0027/README.md) gives traits and implementations
type parameters and states the overlap rule between a generic and a concrete
implementation. Two are a compile error
naming both, across the standard library, the imported modules and the
program; import order never chooses, and the one implementation reached
through two imports is still one. An implementation is declared by the unit
that declares the trait or the type, so a third party cannot change the
meaning of `Add for Int`: a program implements its own trait for any type and
any trait for its own type.

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

Built: `trait` and `impl` with `Self` and type parameters,
`impl<T> Index<T> for Seq<T>`, default bodies, `T : Trait` and
`E : Index<T>` bounds checked at every call with open parameters inferred,
dispatch on the value's own type, operators resolving through `Add`, `Sub`,
`Mul`, `Div`, `Rem`, `Eq`, `Ord` and `Index`, native headers for the
built-in types in `std/core/mod.hql`, coherence and ownership as above. The core
set is `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Index`; `Neg`,
`Hashable` and `Numeric` are not declared.

| Type | Native implementations |
| --- | --- |
| `Bool` | `Eq` |
| `Int`, `Float` | `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem` |
| `String`, `Date` | `Eq`, `Ord` |
| `Seq<T>`, and so `List<T>` and `Ranking<T>` | `Index<T>` |
| `Data` | `Index<Data>` |

`Orderable` stays as the ancestry form of `Ord`: the built-in ordered types
name it in their supertype sets, and a type implementing `Ord` adopts it, so
`compare`, `<`, `sort`, `min`, `max` and `SortedMap` keys accept either. The
[type system card](../../../wiki/hql/type-system.hmd) states the split between
ancestry, traits and registrations.

## Work units

1. The grammar for `trait`, `impl` and `Self`. Done.
2. Trait declarations, implementations, constraint checking at a call and
   the coherence diagnostic. Done.
3. The core trait set in `std/`, with native implementations for `Int`,
   `Float`, `String`, `Bool` and `Date` declared as bodyless headers. Done;
   `Neg`, `Hashable` and `Numeric` remain.
4. The type system card's traits section. Done.
