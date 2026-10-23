# HQL-0016: Type tests, narrowing, widening and conversion

- **Status**: implemented
- **Created**: 2026-10-18
- **Source**: [issue 0018](../../issues/0018-operator-synonymy-audit.md)

## Abstract

Four operations relate a value to a type, and each has its own name. `is(U)`
tests the runtime type of a value and returns a `Bool`. `filter` narrows: a
predicate that is an `is(U)` test of the element makes the result's element
type `U`, and `of_type(U) := filter(x => x | is(U))` is the shorthand for that
selection. `typed` is removed, with no alias. Widening to a supertype needs no
operation: a value of a subtype is usable wherever its supertype is expected,
and a binding annotated with the supertype is proven by the checker and
builds nothing. Conversion is
the constructor of a concrete collection type, `List(xs)` and `Set(xs)`,
which HQL already has; a constructor is also a pipeline step, so `xs | Set` is
`Set(xs)`. `list` ([HQL-0007](../HQL-0007/README.md)) is retired as a second
name for `List`, and `List` stops accepting a `Set`. There is no cast.

## Motivation

- `typed` names the operation by what its result is, not by what it does, and
  it does two things at once: it tests each element and it narrows the result.
  The test on its own, a `Bool`, has no name, and a program writes a two-arm
  `match` for it.
- Narrowing and widening are opposite directions and differ in every property
  a program relies on: narrowing is checked at run time and can drop a value,
  widening is checked before the program runs and keeps every value. A cast
  joins them under one name.
- Concrete collection types already have constructors
  ([expressions](../../models/behavior/expressions.md)): `Set(sightings)`
  collapses duplicates in
  [`examples/birds/`](../../../examples/birds/queries/sightings.hql). `list`
  has the same signature and result as `List`, which the rule in
  [issue 0018](../../issues/0018-operator-synonymy-audit.md) does not admit.

## Non-goals

- **No unchecked narrowing.** There is no `unwrap` in HQL, and narrowing does
  not add one. A value that may not be a `U` is selected, matched or tested,
  never asserted.
- **No scalar conversion.** A constructor changes the container and keeps the
  elements. Turning a `String` into an `Int` is parsing, which can fail and
  belongs to a separate operation.
- **No conversion by annotation.** `xs : List<Card> = seq` is a type error.
  An annotation widens and never builds a value, so the cost of building one
  is written where it is paid.
- **No general flow typing.** Narrowing reads one predicate form inside
  `filter`; `given`, `match` guards and `&&` are not narrowed by it.

## Specification

### 1. `is(U)` tests a type

`is(U)` returns whether a value is of the type `U`.

```hql
is<T, U>(x: T, type: Type<U>) : Bool
```

- `x | is(U)` MUST return `true` when the runtime type of `x` is `U` or a
  subtype of `U`, and `false` otherwise.
- `is(U)` MUST be refused when no `T` can be a `U` and no `U` can be a `T`:
  no `T` can be a `U`, so this is always false.
- When `T` is already a `U`, the checker SHOULD warn that the test is always
  true.
- `U` MAY be a bare constructor, `is(List)`, which tests the container and
  leaves its argument open.

A `Data` value is tested by its shape, and the value a test keeps or binds
is read at the type it passed:

- A leaf is a `U` when the scalar it holds is, and is read as that scalar.
- A sequence is a `List<U>` when every item is a `U`, and is read as that
  list.
- A map is a record `R` the program declares without type parameters when it
  has an entry for every required field of `R` and each entry is read at its
  field's type. An optional field without an entry, or with an empty one, is
  absent. An entry `R` does not name makes the map not an `R` when `R` is
  closed, and is kept beneath the view when `R` narrows `Data`
  ([HQL-0030](../HQL-0030/README.md)). The map is read as the `R`
  the constructor builds from those fields.
- `is(R)` over a `Data` value MUST NOT be refused as always false when `R` is
  such a record. A built-in record, a card kind among them, is established
  by its source and MUST NOT be read from a tree, so `d | is(Card)` is
  refused.

The longhand is a two-arm `match`, which admits `is` as a shorthand under
[HQL-0015](../HQL-0015/README.md):

```text
match x { U _ => true, _ => false }         x | is(U)
```

### 2. `filter` narrows, and `of_type(U)` is its shorthand

A predicate that is an `is(U)` test of the element narrows the result.

```hql
filter<F<_> : Iterable<_>, T, U>(xs: F<T>, by: x => x | is(U)) : F<U>
filter<S : Iterable<T>, T>(xs: S, by: T -> Bool)              : S
```

- The checker MUST select the narrowing overload when the lambda's body is
  `p | is(U)` or `is(p, U)` and `p` is the lambda's parameter. Any other
  predicate selects the ordinary overload, which keeps the element type.
- The narrowing overload keeps exactly the elements the ordinary one keeps;
  only the static type differs.
- `filter` over an `Option<T>` ([`std/core/mod.hql`](../../../std/core/mod.hql))
  narrows the same way, so `o | filter(x => x | is(Card))` is an
  `Option<Card>`. A single value that is not an option is narrowed with
  `match`.

`of_type(U)` names the selection:

```hql
of_type(U) := filter(x => x | is(U))
```

- `of_type` is defined in `std/collections/mod.hql`. Its parameter `U` is a type
  argument, which extends the parameters of HQL-0015 with a type form: a
  sugared parameter written as a type name binds a type.
- `cards | of_type(RelationCard)` returns a `Set<RelationCard>` from a
  `Set<Card>`, and `[1, "owl", 2] | of_type(Int)` returns `[1, 2]`.

### 3. `typed` is removed

- `typed` MUST be an unknown operation, and its diagnostic MUST name
  `of_type`. There is no alias and no deprecation period.
- Every program, test and document that calls `typed(U)` calls `of_type(U)`.
- HQL-0015's shorthand table replaces its `typed(U)` row: the selection's
  longhand is `filter(x => x | is(U))`, and `is(U)` is a shorthand of a
  `match`.

### 4. Widening needs no operation

A value of a subtype is usable wherever its supertype is expected, so no step
widens.

- An annotated binding to a supertype MUST be proven by the checker and MUST
  NOT copy, convert or reorder the value: `s : Seq<Hit<Card>> = ranking`
  holds the same hits.
- An annotated binding to a type the value is not a subtype of MUST be
  refused, naming `of_type` when the annotation is a subtype of the value's
  type, and the constructor when it is another container.
- A conformance check needs no step either: `cards | sort` is refused where
  `Card` is not `Orderable`, at the step that needs the contract.
- `sort` over a `Ranking<T>` returns a `Seq<Hit<T>>`: a new order is not the
  retrieval's order, so the result is not a ranking.
  [Issue 0018](../../issues/0018-operator-synonymy-audit.md) carries this into
  `sort`'s signature.

### 5. Conversion is construction

The constructor of a concrete collection type builds a value of that type
from the elements of its input.

```hql
List<T>(xs: Seq<T>)        : List<T>
Set<T>(xs: Collection<T>)  : Set<T>
```

- `List(xs)` MUST return the sequence's values at their positions. Over a
  `List<T>` it is an identity and copies nothing.
- `List` MUST NOT accept a `Set<T>`: a set has no order, and a list built from
  one would expose an iteration order the
  [collection types](../../wiki/hql/types/collections/collection-types.md)
  leave unspecified. The message names `sort`, which orders a `Collection<T>`
  into a `List<T>`.
- `List` MUST NOT accept a `Stream<T>` that is not a `Seq<T>`: the stream may
  not end. The message names `take` and `collect(n)`.
- `Set(xs)` MUST return the distinct values of its input under HQL equality.
  Over a `Set<T>` it is an identity.
- An abstract type is refused, as it is today: there is no single value of an
  abstract type to build. `Ranking` is not constructed from elements; a
  retrieval makes it, with its query and scores.
- The map constructors, `Map(keys, values)`, `OrderedMap` and `SortedMap`,
  are unchanged.

A constructor is a pipeline step. The piped value is its first argument:

```text
x | C          ≡  C(x)
x | C(a)       ≡  C(x, a)
```

`xs | Set` is `Set(xs)`, and `keys | Map(values)` is `Map(keys, values)`.

### 6. `list` is retired

- `list` MUST be an unknown operation, and its diagnostic MUST name `List`.
- HQL-0007's materialization step is `List(xs)`, or `xs | List` in a
  pipeline, with the refusals of *Conversion is construction*.
- No shorthand replaces it: `List` is already as short, and two names for one
  signature are not admitted.

### 7. Side by side

| | `is(U)` | `of_type(U)` | a binding `x : U` | `List(xs)`, `Set(xs)` |
| --- | --- | --- | --- | --- |
| Does | tests | selects and narrows | widens | builds |
| Direction | none | down | up | to another container |
| Checked | at run time | at run time | before running | before running |
| Can drop values | no | yes | no | `Set` drops duplicates |
| Builds a value | no | yes, the selection | no | yes, except the identities |

## Backwards Compatibility

Breaking. `typed` is removed with no alias; its uses in `tests/`, in the
[type interpretation](../../wiki/hql/type-interpretation.hmd) card, in
HQL-0012, HQL-0014 and HQL-0015, and in issues 0015, 0016 and 0018 change to
`of_type`. `List(set)` stops checking, and a program sorts the set instead.
`list` is refused naming `List`, and a constructor is a pipeline step, so a
program that wrote `xs | list` writes `xs | List`. `is` and `of_type` are new
names.

## Reference Implementation

- `src/extensions/collections.rs` — `is`; `typed` removed with its
  diagnostic; the narrowing overload of `filter`; `sort` over a `Ranking`
  returning a `Seq`
- `src/checker.rs` — selecting the narrowing overload from the predicate's
  form; the refusals of an annotated binding
- `src/types/value.rs`, `src/types/program.rs` — the structural test of a
  `Data` tree, reading a map as a declared record
- `src/constructors.rs` — `List` refusing a `Set` and an open `Stream`; a
  constructor as a pipeline step
- `std/collections/mod.hql` — `of_type`, and the constructor signatures
- `doc/wiki/hql/type-interpretation.hmd` — `is`, `of_type`, widening by a
  binding, and construction
- `doc/wiki/hql/types/collections/collection-types.md` — construction as the
  conversion between collections
- `doc/models/behavior/expressions.md` — the constructor in pipeline position
- `doc/models/behavior/queries.md` — `List` in place of `list`

## Test Plan

- `card | is(Card)` is `true` with a warning; `doc | is(Card)` is `true` for a
  card and `false` for another doc; `1 | is(Card)` refused as always false
- `cards | filter(c => c | is(RelationCard))` is a `Seq<RelationCard>`, and
  `cards | filter(c => c.name == "x")` a `Seq<Card>`
- `cards | of_type(RelationCard) | count` equal to the count `typed` gave in
  `tests/vault.rs`
- `[1, "owl", 2] | of_type(Int)` is `[1, 2]` at `List<Int>`
- `some(doc) | filter(x => x | is(Card))` is an `Option<Card>`
- `typed(Card)` refused naming `of_type`; `list` refused naming `List`
- `s : Seq<Hit<Card>> = ranking` holds the same hits; `c : Card = doc`
  refused naming `of_type`; `xs : List<Card> = seq` refused naming `List`
- `ranking | sort(by = h => h.doc.name)` is a `Seq<Hit<Card>>`
- `xs | List` equal to `List(xs)`; `xs | Set` equal to `Set(xs)`;
  `keys | Map(values)` equal to `Map(keys, values)`
- `List({1, 2})` refused naming `sort`; `List` over an open stream refused
  naming `take`
- `[1, 2, 2] | Set` of size 2
- `{title: "Dune", pages: 412}` as `Data` is a `Book`, and `of_type(Book)`
  over a `List<Data>` keeps it as `Book(title = "Dune", pages = 412)`; a
  tree without `pages`, or with a text `pages`, is not; `d | is(Card)` is
  refused as always false

## Changelog

- 2026-10-18: drafted
- 2026-10-19: accepted; `is`, `of_type`, the narrowing `filter` including over
  an `Option`, `is` over a bare constructor, the always-true warning, the `List`
  and `Set` constructors with their refusals, and conversion hints on a refused
  annotation are implemented, and record values and the conversions that need
  them are not yet
- 2026-10-22: implemented; a `Data` map is read as a record the program
  declares by its shape, through `is`, `of_type` and a type arm of `match`
