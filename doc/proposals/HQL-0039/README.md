# HQL-0039: Traits, validator traits, a trait's type, and traits a module implements

**Status**: drafted
**Created**: 2026-10-23
**Source**: [HQL-0024 part 5](../HQL-0024/parts/05-traits.md),
[HQL-0027](../HQL-0027/README.md), [HQL-0034](../HQL-0034/README.md), and the
receiver modules of [HQL-0024 part 4](../HQL-0024/parts/04-receiver-modules.md)

## Abstract

A trait is a compile-time contract: a named set of function signatures over
`Self`, declared in a module's `mod.hql`, that a type satisfies through one
implementation. A trait is not a type. Its name appears only in a fixed set
of positions: a bound, an implementation, an extension, a receiver module's
trait list, and the two validator positions. Its declaration MAY name a type:
`trait Shape as Figure` declares `Figure`, the union of the types that
implement `Shape`. There are three kinds of trait,
all one construct: capability traits, operator traits and validator traits.
A validator trait, which extends `Validator`, is the form HQL-0034 calls a
validator type. A field `f: V` declared with one is of the type that
implements `V`, held to `V`'s check. This record adds a second way to declare
an implementation: a receiver module states the traits its receiver type
implements, `module string for String : Index<String>, Slice`, and its own `self`
functions are that implementation.

## Motivation

- Trait rules are split across three records and a section of the type
  system card, and no single text says what a trait is not.
- HQL-0034 calls a trait extending `Validator` a "validator type" and lets
  it stand in a field's type position. Read alone, that says a trait is a
  type, which part 5 denies.
- A receiver module holds a type's functions, and a separate `impl` states
  that the type has a capability. `String`'s positions and parts are
  `impl Index<String> for String` and `impl Slice for String` headers in
  `lib/std/core/mod.hql`, while its other text functions live in
  `lib/std/string/mod.hql`. One type's functions and capabilities belong in one
  module.
- Strings, sequences and streams share capabilities, but each has its own
  receiver module. A trait list on a receiver module states which capability
  each one has, in the place the functions are written.
- Values of several types that share a capability have no common type. A
  list of them needs a union declared by hand, which restates the
  implementing types, and a `match` whose every arm forwards the same call.

## Non-goals

- Trait objects, existential types, associated types, `derive`, or dynamic
  registration. None of these exist, and this record adds none. A trait's
  named type is a union of declared types, not an existential.
- New standard traits. Which capabilities of strings, sequences and streams
  become traits is a later record. This one supplies the mechanism.
- A receiver pattern over a capability, `module math for T where T : Numeric`,
  which part 4 does not admit.

## Specification

### 1. What a trait is

A trait is a named contract over `Self`: required function signatures and
optional default bodies.

```hql
trait Shape {
    fn area(self) : Float
    fn name(self) : String
    fn describe(self) : String = fmt("{} with area {}", self.name(), self.area())
}
```

- A trait MUST NOT be the type of a value, a binding, a parameter or a
  result. `s: Shape` as a parameter is refused, with a diagnostic naming the
  bound form `<T : Shape>(s: T)`. The one exception is the validator field described
  under "Validator traits". A type that a trait's declaration names with
  `as` is a type of its own (see "A trait's type").
- A trait is checked at compile time. Calling a trait function dispatches on
  the value's own type at run time, so a value held at a wider type still
  takes its own implementation.
- A trait MAY take type parameters, `trait Index<T>`
  ([HQL-0027](../HQL-0027/README.md)). It has no associated types.

### 2. Where a trait's name appears

A trait's name is written in exactly these positions.

| Position | Form | Meaning |
| --- | --- | --- |
| declaration | `trait A { … }` | declares `A` |
| declaration with a type | `trait A as F { … }` | declares `A`, and the type `F` (see "A trait's type") |
| extension | `trait A : B, C { … }` | `A` has the functions of `B` and `C` as its own |
| bound | `fn f<T : A>(x: T)` | every call's `T` implements `A`; `x.g()` resolves through `A` |
| implementation | `impl A for T { … }` | `T` implements `A` |
| module trait list | `module m for T : A, B` | `T` implements `A` and `B` through the module's functions (see "Traits a module implements") |
| validator rule | `validate f with V` | `f`'s value passes `V` (see "Validator traits") |
| validator field | `f: V` | `f` is of the type that implements `V` and passes `V` (see "Validator traits") |

- A trait's name written in any other type position is refused, unless the
  trait names its type with its own name, `trait A as A`: that name in a type
  position is then the type.

### 3. Kinds of trait

Every trait is the one `trait` construct. Its kind is determined by its uses
and by what it extends, never by a keyword.

| Kind | Defined by | Examples |
| --- | --- | --- |
| capability trait | named in bounds by generic functions | `Shape`, `Scale<F>` |
| operator trait | a core trait that an operator resolves through | `Eq` for `==`, `Ord` for `<` and `sort`, `Add` for `+`, `Index<T>` for `xs[i]`, `Slice` for `xs[a:b]` |
| validator trait | has `Validator` among the traits it extends | `Email`, `NonEmpty` |

- A trait MAY be of more than one kind. A validator trait MAY also be named
  in a bound.
- The operator traits are the core traits declared in `lib/std/core/mod.hql`.
  A program MAY implement them for its own types, and MUST NOT declare new
  operator traits.

### 4. Extension

`trait A : B` makes the required functions and default bodies of `B` the
functions of `A`.

- An implementation of `A` supplies the functions of `A` and of every trait
  `A` extends. It does not implement `B` by doing so.
- Extension is transitive: a trait extending `A` has `B`'s functions as well.

### 5. Validator traits

A validator trait is a trait that extends `Validator`, whose
`validate(self) : Bool` is a check a value passes. This record renames
HQL-0034's "validator type" to validator trait. HQL-0034's rules hold
unchanged, read as follows:

- `validate f with V` holds where `V`'s `validate` of `f`'s value is `true`.
  It is a type error where `f`'s type does not implement `V`, or where `V` is
  not a validator trait.
- A field `f: V` declares a **refinement**, not a field of trait type. The
  field's type is the one type `T` that implements `V`, and `r.f : T`. The
  record holds the field to `V`'s `validate` wherever it is read or built.
- A validator trait used as a field's type MUST have exactly one
  implementing type. With none, or with several, it is refused there and
  named in a rule instead.

```hql
trait Email : Validator { }

impl Email for String {
    fn validate(self) : Bool = self.contains("@") and not self.starts_with("@")
}

type UserData : DataContract = {
    work_email: Email              // a String, held to Email
    contact?: String
    validate contact? with Email   // the same check, as a rule
}
```

### 6. Traits a module implements

A receiver module MAY list traits after its pattern. Its `self` functions
are then the implementation of those traits for the pattern's type.

```hql
// lib/std/string/mod.hql
module string for String : Index<String>, Slice
import core

fn at(self, position: Int) : Option<String>
fn slice(self, range: Range) : String
fn trim(self) : String                      // a receiver function only
```

```hql
// shapes/circle.hql: a workspace module declaring its own type
module shapes.circle for Circle : Shape

type Circle { radius : Float }

fn area(self) : Float = 3.0f * self.radius * self.radius
fn name(self) : String = "circle"
```

- `module m for P : A, B` declares one implementation of each listed trait for
  `P`, with the same meaning as `impl A for P` with the module's functions as
  the body.
- For each listed trait, each required function MUST be a `self` function of
  the module whose signature matches with `Self = P`, or else MUST have a
  default body. A missing or mismatched function is refused against the
  `module` line, and the diagnostic names the trait and the function.
- A function that is listed in no trait's signature remains an ordinary
  receiver function.
- A trait function the module supplies is one declaration. `s.at(1)`,
  `string.at(s, 1)` and `s[1]` all resolve to it.
- Two listed traits that require a function of the same name are refused,
  because one `self` function cannot be two implementations. Such traits are
  implemented with `impl`. Every validator trait requires `validate`, so a
  module lists at most one validator trait.
- A pattern's variables bind the trait's arguments, so
  `module seq for Seq<T> : Index<T>, Slice` implements `Index<T>` for every `T`.
- A module that an extension provides MAY write the functions without bodies,
  which makes a native implementation bound like any other
  ([HQL-0024 part 12](../HQL-0024/parts/12-contracts-and-binding.md)). This
  replaces a bodyless `impl` header.

### 7. One implementation, either declaration

`impl` and a module's trait list both declare implementations, and both fall
under one coherence rule.

- For one effective environment, there is at most one implementation of a
  trait, at one set of its arguments, for one type, whether it is declared by
  `impl` or by a module's trait list. Two are an error naming both
  declarations ([HQL-0024 part 5](../HQL-0024/parts/05-traits.md),
  [HQL-0027](../HQL-0027/README.md)).
- An implementation is declared by the unit that declares the trait or the
  type, by either form.
- `impl` remains the form for implementations that a receiver module cannot
  state: a type that has no receiver module, traits whose functions share a
  name, and validator traits.

### 8. Where a trait is declared

A trait belongs to a module and is declared in that module's `mod.hql`.

- A library trait MUST be declared in the `mod.hql` of the module whose
  domain it states, and is visible where that module is imported
  ([HQL-0028](../HQL-0028/README.md)). The core traits are declared in
  `lib/std/core/mod.hql` and are in scope everywhere. The standard validator
  traits are declared in `lib/std/validators/mod.hql`.
- A trait declared in a program belongs to the program module and is visible
  in that program only.
- A trait and the receiver modules implementing it MAY be declared in
  different modules, subject to the ownership rule of "One implementation, either declaration".

### 9. A trait's type

The types that implement a trait form a union. The union has no name unless
the trait's declaration gives it one with `as`.

```hql
trait Shape as Figure {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

figures : List<Figure> = [Rect(width = 2.0f, height = 3.0f), Circle(radius = 1.0f)]

figures | map(f => f.describe())    // ["area 6.0", "area 3.0"]
```

- `trait A as F` declares the trait `A` and the type `F`. `F` is a type in
  every type position: of a value, a binding, a parameter, a result and an
  element.
- The members of `F` are the types that implement `A` in the effective
  environment, by `impl` or by a module's trait list: the set the coherence
  rule of "One implementation, either declaration" ranges over. A value of
  `F` is a value of one member.
- `F` is a union and is checked as one. A literal is held at `F` by an
  annotation, as at any union. A `match` over `F` binds each member at its
  own type and MUST cover every member, so an implementation for a further
  type makes a `match` without a wildcard arm incomplete.
- A function of `A`, its own or one of a trait it extends, is a function of
  `F`. `f.g()` dispatches on the value's own type to that type's
  implementation.
- `F` implements `A`, so `F` satisfies a bound `T : A`. The implementation
  is the union's own, and `impl A for F` is refused.
- A trait that names its type MUST write `Self` only as the receiver, in its
  own functions and in those of every trait it extends. Any other `Self`, a
  parameter `other: Self` or a result `Self`, is refused against the
  declaration, and the diagnostic names the function. Two values held at `F`
  may be of two members, so a function relating two values of `Self` has no
  meaning over `F`.
- A trait that names its type MUST NOT take type parameters, and MUST NOT be
  a validator trait. An implementation with type parameters,
  `impl<T> A for Ring<T>`, of a trait that names its type is refused.
- `F` MAY be the trait's own name, `trait Named as Named`. A trait position,
  a bound, an implementation, an extension or a module trait list, reads the
  trait; a type position reads the type. A validator trait is the one trait
  written in a type position, and it never names its type, so the two
  readings never meet.
- `F` is visible where `A` is, and is private where `A` is private.

## Backwards Compatibility

Every `trait` and `impl` that checks today checks unchanged. `module m for P`
without a trait list keeps its meaning. A parameter, binding or result typed
by a trait is refused, which no existing example writes. "Validator type"
becomes "validator trait" in HQL-0034, in the type system card and in the
comments of `lib/std/core/mod.hql` and `lib/std/validators/mod.hql`. Their behaviour
is unchanged. `as` after a trait's name is new syntax, and a trait without it
declares no type.

## Reference Implementation

- `src/parser.rs`: the trait list after `module <name> for <pattern>`, and
  `as <Name>` after a trait's name
- `src/checker.rs`, `src/declarations.rs`: implementations collected from a
  module's trait list into the same registry as `impl`; the missing-function,
  shared-name and coherence diagnostics; refusal of a trait in a type
  position other than a validator field
- `src/checker.rs`: a trait's named type as the union of the trait's
  implementing types, its own implementation of the trait, and the
  diagnostics for `Self` outside the receiver, a type parameter, a validator
  trait, a generic implementation and `impl A for F`
- `src/extensions/binding.rs`: native implementations from a module's
  bodyless functions
- `lib/std/string/mod.hql`, `lib/std/seq/mod.hql`: `Index` and `Slice` stated in the
  modules' trait lists, and the corresponding headers removed from
  `lib/std/core/mod.hql`
- `doc/wiki/hql/type-system.hmd`: its traits section rewritten around this
  record, or moved to a card of its own, `hql/traits.hmd`

## Test Plan

Proposed cases under `examples/`, with `status: proposed` and
`proposal: HQL-0039`, promoted as each one checks:

- `module shapes.circle for Circle : Shape` in a workspace module, called through a bound
  and as `c.area()`
- `module string for String : Index<String>, Slice` giving `s[1]`, `s[1:3]` and
  `s.at(1)` as one function
- a default body inherited through a module's trait list
- a validator field `f: Email` and the rule `validate f with Email`, both
  named validator traits
- invalid: a module listing a trait whose required function it lacks
- invalid: a module listing two traits that require the same function
- invalid: `impl Shape for Circle` beside `module shapes.circle for Circle : Shape`
- invalid: a parameter typed by a trait
- `trait Shape as Figure`: a `List<Figure>` holding two implementing types,
  `f.describe()` through it, `Figure` satisfying `T : Shape`, and a `match`
  over its members ([`examples/traits/`](../../../examples/traits/README.md))
- `trait Named as Named`: one name read as the trait in a bound and as the
  type elsewhere
- invalid: a trait naming its type with `other: Self` in one of its functions
- invalid: a `match` over a trait's named type missing one implementing type
- invalid: `impl Shape for Figure`

```bash
cargo test --locked --test traits --test corpus --test structural --test extension_contracts
```

## Changelog

- 2026-10-23: drafted
