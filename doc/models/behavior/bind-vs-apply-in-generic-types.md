# Generic brackets: bind and apply

Status: **decided.** Generics are written with angle brackets, in every
position. Square brackets are value-level and never appear in a type.

```hql
fn first<T>(xs: Seq<T>) : T?
```

## The rule

```text
< >   the type world — binding a parameter and applying a constructor
[ ]   the value world — sequence literals and indexing
```

One symbol, one world. There is no second spelling for either job:

```hql
Seq<Concept>                        // apply a constructor
Map<String, Data>
Relation<Person, Project>
Renderable<T>

fn map<T, U>(xs: Seq<T>, f: T -> U) : Seq<U>    // bind parameters
fn resolve<T>(x: Ref<T>) : T?

[alice, bob, charlie]               // a sequence value
{alice, bob, charlie}               // a set value
xs[0]                               // indexing
```

## Why angle brackets

HQL wants `[]` at the value level, for sequence literals and for indexing. If
`[]` also meant type application, one pair of brackets would carry four
conceptually different jobs:

```hql
[alice, bob]        // sequence literal
xs[0]               // indexing
Seq[Person]         // type application
fn foo[T](...)      // generic declaration
```

A parser can tell those apart — Scala and Python prove it. The question is
whether a *reader* should have to. With angle brackets reserved for types, an
angle bracket always means the type world and a square bracket always means the
value world, and neither ever has to be disambiguated by context.

The spelling is also the conventional one. `Seq<T>` and `fn first<T>` read the
same way in C++, Java, C#, TypeScript, Rust and Kotlin, so nothing here has to
be learned before it can be read.

## The hybrid is rejected

One arrangement was considered and turned down: `[...]` to **bind** type
variables and `<...>` to **apply** a constructor.

```hql
fn map[T, U](xs: Seq<T>, f: T -> U) : Seq<U>     // rejected
fn resolve[T](x: Ref<T>) : T?                    // rejected
```

The distinction is coherent — binding and application genuinely are different
operations — but it buys a reader nothing they did not already know from
position, and it costs the property the whole choice exists to secure: that a
bracket shape tells you which world you are in. It also reintroduces the
overload of `[]` that motivated angle brackets in the first place. **No
hybrids.**

## Syntax migration must preserve semantics

Adopting this across `doc/**` was a mechanical rewrite, and a mechanical rewrite
is allowed to change spelling and nothing else. `Set<T>` for `Set[T]` is such a
change. This is not:

```hql
card.metadata.concepts : List<Concept>
```

`List` is withdrawn — see [collections](../../wiki/hql/collections.hmd), *There
is no `List`* — so the occurrence cannot simply be respelled. It needs a
semantic choice between `Seq<Concept>` and `Set<Concept>`, and that choice
follows from whether the order of a card's concepts carries information, which
is a question about what `concepts` means.
It was therefore left unresolved in [Card](../../wiki/hql/types/card-type.hmd)
rather than settled in passing.

The separation matters more than the bracket choice itself: a syntax migration
that quietly decides semantics leaves no record that a decision was made.

## Consequences

- [type-system](../../wiki/hql/type-system.hmd) states the rule and no longer
  lists bracket style as an open question.
- [core](../../wiki/hql/core.hmd) and
  [design-status](../../wiki/hql/design-status.hmd) restate it; the vocabulary
  tables are `Set<T>`, `Seq<T>`, `Map<K, V>`.
- Carrying the spelling through every file is tracked as `CON-13` in
  [consolidation](../../status/consolidation.md). The example corpus is not in
  this working tree and is the outstanding residue.
- Quotations that are evidence of the former divergence keep their original
  spelling: `INC-06` in [inconsistencies](../../status/inconsistencies.md)
  quotes `Option<List[Card]>`, which is the finding.

## What this does not decide

- **`List` versus `Seq` versus `Set`** at any particular site, including
  `card.metadata.concepts` — `INC-05` and the note on
  [Card](../../wiki/hql/types/card-type.hmd).
- **Whether type arguments are ever written at a call site.** `validate<Person>(card)`
  is one candidate spelling for explicit application; inference is the other.
  [type-system](../../wiki/hql/type-system.hmd) records that as open.
- **Bound syntax.** `fn show<T, R>(x: T) where T <: Renderable<R>` and the
  inlined `fn show<T <: Renderable<R>, R>(x: T)` are both open; that the
  relation is spelled `<:` in either is not.
- **Variance**, which depends on whether constructed values stay immutable.

See [type-system](../../wiki/hql/type-system.hmd) for generics and subtyping,
[collections](../../wiki/hql/collections.hmd) for the collection vocabulary, and
[optional fields and optional values](options-in-types-and-fields.md) for the
other spelling question settled alongside this one.
