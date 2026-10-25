# HQL-0027: Generic traits: type parameters on `trait` and `impl`

**Status**: implemented
**Created**: 2026-10-22
**Source**: [issue 0032](../../issues/archive/0032-generic-traits.md), the
[generic traits example corpus](../../../examples/generic-traits/README.md)

## Abstract

A trait takes type parameters, `trait Index<T>`, and an implementation takes
its own and names the trait's arguments, `impl<T> Index<T> for List<T>`. A
bound names a generic trait with arguments, `E : Index<T>`, and a call infers
the arguments it leaves open from the one implementation the receiver's type
has. Coherence covers the trait, its arguments and the implementing type
together, and two implementations that could apply to one type at one
argument list are refused, generic or concrete. A trait parameter is the
only way a trait names another type: there are no associated types, and a
parameter that the implementing type decides is inferred, not declared
apart. The codec contracts become traits over the format, `impl
Decoder<String> for Lines`, and a declared type takes `xs[i]` by implementing
`Index<T>`.

## Motivation

A trait declares functions over `Self` and the concrete types it names, so a
capability whose functions return or take another type cannot be declared.
Four contracts stay outside the trait system because of it:

| Contract | Today | What it needs |
| --- | --- | --- |
| A position, `xs[i]` | language-defined over the built-in types; a declared type is "not indexed" | the element type `T` in `at(self, position: Int) : Option<T>` |
| A codec | `abstract type Decoder<F, T>` and `Encoder<T, F>` in `lib/std/format/mod.hql`, related to the Rust tables by comment | a relation between a format and a value type, one format to several types |
| A rendering | `Renderable<T>`, a registration with no declared shape | the same relation as an encoder |
| A generic built-in type | `impl Slice for List<T>` in [HQL-0026](../HQL-0026/README.md) has nothing to bind `T` | a parameter on the implementation |

## Non-goals

- Associated types. A parameter the implementing type decides is a trait
  parameter that inference fills; see [outputs are
  parameters](#7-outputs-are-parameters).
- Specialization. An implementation never overrides a more general one.
- Trait objects and dynamic dispatch through a trait. A trait keeps no
  runtime representation, as [HQL-0024, traits](../HQL-0024/parts/05-traits.md)
  has it.
- A trait function without `self`. A codec trait has no functions until a
  format is a value a program can hold.
- A codec written in HQL. The codec traits are implemented natively; an HQL
  `impl Encoder<T> for …` waits for the item above.

## Specification

### 1. Grammar

```text
trait      = "trait" Name [ parameters ] "{" { method } "}"
impl       = "impl" [ parameters ] Name [ arguments ] "for" type [ "{" { method } "}" ]
parameters = "<" parameter { "," parameter } ">"
parameter  = Letter [ ":" bound ]
arguments  = "<" type { "," type } ">"
bound      = type
```

- A parameter is one capital letter, as a function's is, and a trait's or an
  implementation's parameters are distinct.
- `<` after `impl` opens the implementation's parameters; `<` after the trait
  name opens the trait's arguments.
- A trait MAY have no functions: `trait Decoder<T> {}` is a relation between
  types that the language's own operations consult.

### 2. A trait's parameters

```hql
trait Index<T> {
    fn at(self, position: Int) : Option<T>
}
```

- A trait parameter MAY appear in any parameter or result of the trait's
  functions, beside `Self`.
- A parameter's bound, `trait Decoder<T : Data>`, is checked against every
  argument an implementation or a bound gives it.
- A trait named without arguments where it has parameters, or with a number
  of arguments other than its parameters, is a type error: "`Index` takes 1
  type argument, not 0".

### 3. An implementation's parameters

```hql
impl<T> Index<T> for Ring<T> {
    fn at(self, position: Int) : Option<T> = self.items[position % length(self.items)]
}
impl Index<Data> for Data
```

- An implementation MUST give the trait one argument per parameter.
- Each of an implementation's parameters MUST appear in its implementing type
  or in the trait's arguments; one that appears in neither is a type error:
  "`T` of this impl appears in neither the type nor the trait's arguments".
- Inside the implementation, the trait's functions are written with `Self` as
  the implementing type and each trait parameter as its argument: the
  signature `at(self, position: Int) : Option<T>` of `Index<T>` is
  `at(self, position: Int) : Option<T>` for `Ring<T>`, with the
  implementation's `T`.
- A native header takes parameters as an HQL implementation does,
  `impl<T> Index<T> for Seq<T>`.
- The ownership rule of [HQL-0024,
  traits](../HQL-0024/parts/05-traits.md#coherence) stands: an implementation
  belongs with its trait or its implementing type. A trait argument does not
  make an implementation a unit's own.

### 4. Bounds over a generic trait

```hql
fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]
```

- A bound MAY give a generic trait arguments that name the function's own
  parameters, in any order of declaration.
- Inside the body, a value of `E` takes the trait's functions with its
  arguments in place: `xs.at(0)` is `Option<T>`, and `xs[0]` resolves
  through `Index<T>`.
- At each call, the bound MUST hold for the types the call fills in: some
  implementation of the trait applies to the filled implementing type at
  the filled arguments, or the call is a type error: "`first_of` needs E :
  Index<T>, and Int does not implement `Index<T>`".

### 5. Coherence

An implementation is identified by its trait, its trait's arguments and its
implementing type.

- Two implementations of one trait MUST NOT overlap: they overlap when one
  assignment of their parameters makes their implementing types and their
  argument lists the same. Overlap is refused whichever is generic:
  `impl<T> Index<T> for Box<T>` and `impl Index<Int> for Box<Int>` are a
  compile error naming both, as two concrete implementations are today.
- Implementations at different arguments do not overlap: `impl Scale<Int>
  for Vec2` and `impl Scale<Float> for Vec2` are both admitted.
- The scope of the rule is unchanged: the standard library, the imported
  modules and the program together.
- An implementation for a type applies to the types narrowing it. Where
  implementations for two types on one chain of narrowing both apply, the
  narrower type's is the one that applies: a List takes `Seq`'s `Index`.
- An implementation for a union applies to the union's own values, not to
  each member: `Int` narrows `Data`, and `impl Index<Data> for Data` makes a
  document indexed, not an `Int` or a `String`.

### 6. Inference

The arguments a call or an operator leaves open are taken from the
implementation that applies.

- **At a call to a bounded function.** The arguments of the call fill the
  function's parameters first; each bound then names its implementing type,
  and the implementations of its trait that apply to that type are matched
  against the bound's arguments. A parameter the arguments did not fill is
  filled from the one matching implementation:
  `first_of([1, 2])` has `E = List<Int>`, and `impl<T> Index<T> for Seq<T>`
  gives `T = Int`.
- **At a trait function.** `v.at(0)` takes the implementation of `Index` for
  `v`'s type; its result is the trait's result with that implementation's
  arguments in place.
- **At an operator.** `xs[i]` over a declared type resolves `Index<T>` from
  the receiver's type, and is `Option<T>` for the `T` the implementation
  gives.
- When several implementations apply with different arguments, the call's
  argument types choose: `v.scale(2)` takes `Scale<Int>`, `v.scale(2.0)`
  `Scale<Float>`, and evaluation takes the implementation whose parameters
  the passed values fit. When they do not choose, the call is a type error:
  "`convert` of `Convert` is implemented for Celsius at several arguments,
  and this call does not choose between them".
- A generic body reaches an implementation through its receiver, so a bound
  over a trait with functions MUST be met by a type that implements that
  trait once: "`f` needs E : Scale<F>, and Vec2 implements `Scale` at
  several arguments, which a generic body does not tell apart". A trait
  without functions, such as a codec, has nothing to reach and takes no
  such refusal.

### 7. Outputs are parameters

A trait names another type only through a parameter. The evidence is the four
contracts:

| Contract | How many implementations a type has | Shape |
| --- | --- | --- |
| `Index<T>` | one per type: the element type is the type's own | `T` is an output, filled by inference |
| `Decoder<T>`, `Encoder<T>` | several per format: `Json` decodes `Data` and encodes every type | `T` is an input |
| `Renderable<T>` | several per format, as an encoder | it is `Encoder<T>` |
| `impl<T> … for List<T>` | not a trait parameter | an implementation parameter |

An associated type would forbid the codecs' several implementations per
format, which is the reason they need a parameter at all; a parameter with
one implementation per type gives `Index` everything an associated type
would, through [inference](#6-inference). One mechanism covers all four.

### 8. The codec contracts

```hql
trait Decoder<T> {}
trait Encoder<T> {}

impl Decoder<String> for Lines
impl Decoder<Data> for Json
impl<T> Encoder<T> for Json
impl Encoder<Graph> for D2
```

- The format is the implementing type: `Decoder<T>` reads records of `Self`
  as values of `T`, `Encoder<T>` writes values of `T` as records of `Self`.
- `lib/std/format/mod.hql` declares both traits and one header per codec the
  runtime provides. Binding holds the headers and the decoder and encoder
  tables to each other: a format the runtime reads has exactly the decoder
  its header names, and a format with no decoder header is not read; a
  format writes a type exactly when an encoder header covers it.
- `source(e, F)` reads values of the `T` of `F`'s decoder, and `format(x, F)`
  requires an encoder of `x`'s type for `F`. Both keep their checking
  through the tables, which binding proves equal to the headers.
- A registration of a renderer for a type in a format is an implementation
  of `Encoder<T>` for the format: `Renderable<T>` is retired as a separate
  concept.

### 9. Positions over a declared type

`Index<T>` is a core trait in [`lib/std/core/mod.hql`](../../../lib/std/core/mod.hql), beside
the operator traits.

- `Seq<T>` and `Data` implement it natively with bodyless headers; every
  type whose supertype is a `Seq` takes `Seq`'s, `List<T>` and `Ranking<T>`
  among them.
- `xs[i]` over a type implementing `Index<T>` MUST mean `xs.at(i)`.
- The built-in receivers keep the position rules they have, including the
  refusals for a Stream, a Set and a Map, which implement no `Index`.
- [HQL-0026](../HQL-0026/README.md)'s non-goal "an `Index` capability" is
  withdrawn by this record, and its native `Slice` headers are written
  `impl<T> Slice for List<T>`.

## Backwards Compatibility

- `abstract type Decoder<F, T>` and `Encoder<T, F>` become traits of one
  parameter; no program names them.
- Every program that checks today checks with the same types. A declared
  type that was "not indexed" may now implement `Index<T>`.
- `xs.at(i)` checks over a sequence and Data, as the trait function of their
  native implementation.

## Reference Implementation

- `src/ast.rs`: `TraitDecl` and `ImplDecl` gain their parameters and the
  trait's arguments; a bound keeps its arguments.
- `src/parser.rs`: `trait_declaration` and `implementation` read the
  parameter and argument lists.
- `src/checker.rs`: a trait's parameters; an implementation's parameters and
  arguments in `ImplInfo`; bounds with arguments in `trait_bounds` and
  `trait_bounded`; resolution of a bound against the implementations at a
  call, filling open parameters; the overlap check; `indexed` resolving
  `Index<T>`.
- `src/library.rs`: native headers keep their parameters and arguments.
- `src/extensions/traits.rs`: the native `Index` bodies; a native body is
  found for a receiver whose supertype is the header's type.
- `src/types/builtin.rs`: `is_union`, and `takes_implementation_of` for the
  union rule.
- `src/extensions/mod.rs`: binding the codec headers to the decoder and
  encoder tables.
- `src/evaluation.rs`: `xs[i]` over a declared type dispatching to its `at`.
- `lib/std/core/mod.hql`: `trait Index<T>` and its native headers.
- `lib/std/format/mod.hql`: the codec traits and their headers.

## Test Plan

The [generic traits corpus](../../../examples/generic-traits/README.md) pins
every valid case as `status: proposed`; each is promoted to `valid-now` as it
checks. `tests/traits.rs` pins:

- a generic trait declared, implemented generically and concretely, and
  called through a bound, with the open parameter inferred;
- implementations at different arguments chosen by a call's argument types;
- the refusals: overlap between a generic and a concrete implementation, a
  wrong number of trait arguments, an implementation parameter that appears
  nowhere, an unmet generic bound, a call that does not choose between
  implementations, and a bound met by a type implementing the trait twice.

`tests/extension_contracts.rs` pins the codec headers against the tables.

```bash
cargo build --locked
cargo test --locked --test traits --test corpus --test extension_contracts
```

## Changelog

- 2026-10-22: drafted
- 2026-10-22: implemented; the [generic traits
  corpus](../../../examples/generic-traits/README.md) is `valid-now`
