# Subtypes and narrowing

`type A : B` declares that `A` **narrows** `B`: every value of `A` is a
value of `B`, with structure or constraints of its own. The relation reads
from the specific toward the general. Nothing is extended or changed by
writing it: `B` is unaware, and the declaration is a claim about `A` alone.

## A subtype passes as its supertype

A value of the narrower type is usable wherever the wider one is expected,
with no step and no check at run time. A `Person : Named` is a `Named`, and
a list of persons is a list of named things, since a `List` of a subtype is
a `List` of the supertype:

```hql
type Named { name : String }
type Person : Named { born : Int }

fn names(xs: List<Named>) : List<String> = xs | map(n => n.name)

names([Person(name = "Ada", born = 1815)])
```

```text
["Ada"]
```

An annotation widens the same way. `xs : Seq<Int> = [1, 2, 3]` holds the
list at the abstract type `Seq`, and `xs : Collection<Int> = {1, 2, 3}` a
set at `Collection`. The value is the same value; the binding says less
about it.

## An annotation never converts

An annotation to a type the value is not a subtype of is refused. The
message names what would build the value, since building is a cost the
program writes where it is paid:

```hql
xs : Set<Int> = [1, 2, 2]
xs
```

```text
type error: `xs` is declared Set<Int> but its value is List<Int>; a binding never converts, so construct it: `Set(…)`
 --> <expression>:1:17
  |
1 | xs : Set<Int> = [1, 2, 2]
  |                 ^^^^^^^^^
```

The same holds downward: `n : Int = d` over a `Data` value is refused,
because a `Data` is not known to be an `Int`. Going toward the subtype is a
test, below.

## Several supertypes at once

A type may narrow several types, written as a set in braces: `type Int :
{Scalar, Orderable}` in the standard library says an integer is a leaf and
has an order. The set is a conjunction, every parent at once, and differs
from a union, which is a choice. Two abstract types never exclude each
other, since each is satisfied by other types. Two concrete types can share
nothing, and such a set is refused:

```hql
type Impossible : {Int, String}
1
```

```text
type error: nothing is both Int and String, so this supertype set declares a type no value can have
 --> <expression>:1:25
  |
1 | type Impossible : {Int, String}
  |                         ^^^^^^
```

## Abstract types

An **abstract type** is defined by what holds of its values, not by a
representation, and has no values of its own. `Orderable` is the type of
values with a total order; `Scalar` of leaves; `Seq<T>` of finite,
positioned collections, which `List<T>` narrows; `Collection<T>` of finite
collections, which `Set<T>` narrows. An abstract type has no constructor,
and a program declares one with `abstract type`:

```hql
abstract type Shape
type Circle : Shape { radius : Float }
fn describe(x: Shape) : String = "a shape"
describe(Circle(radius = 1.0f))
```

```text
"a shape"
```

An abstract type is where a bound usually points. `T : Orderable` admits
every type with an order: the built-in ordered types declare `Orderable` as
a parent, and a program's type joins them by implementing `Ord`, as
[operators](operators.md) shows.

## Narrowing is a test

Toward the subtype a value goes only by a test, which can fail. `is(U)`
asks whether a value is a `U` and gives a `Bool`. `of_type(U)` keeps the
elements of a collection that are, at type `U`; it is `filter` with an `is`
test, and the narrowing is the checker reading that predicate. A `match`
arm written with a type does the same for one value:

```hql
[1, "owl", 2] | of_type(Int)
```

```text
[1, 2]
```

Over a union, the test selects a member: `shapes | of_type(Circle)` from a
`List<Shape>` is a `List<Circle>`. Over a `Data` tree, the test is by
shape, and the value it binds is read at the type it passed, as
[chapter 18](../18-advanced-data-typing/open-and-closed.md) describes. There
is no assertion: a value that may not be a `U` is selected, matched or
tested, never cast.

## Conversion is construction

Changing the container a value is in is building a new one, and the
constructor of the target type does it. `Set(xs)` holds the distinct
elements of a list, and `xs | Set` is the same call:

```hql
[1, 2, 2] | Set
```

```text
{1, 2}
```

A constructor refuses what its type cannot honestly hold. `List({1, 2})`
is refused, since a set has no order for a list to keep, and the message
names `sort`, which orders a collection into a list. `List` over a stream
that may not end is refused naming `take` and `collect`. A constructor
changes the container and keeps the elements; turning a `String` into an
`Int` is parsing, `text.to_int()`, which gives an `Option` because it can
fail.

## Side by side

| | `is(U)` | `of_type(U)` | a binding `x : U` | `List(xs)`, `Set(xs)` |
| --- | --- | --- | --- | --- |
| Does | tests | selects and narrows | widens | builds |
| Direction | none | down | up | to another container |
| Checked | at run time | at run time | before running | before running |
| Can drop values | no | yes | no | `Set` drops duplicates |
| Builds a value | no | yes, the selection | no | yes |

There is no cast. Each of the four has its own name because each has its
own properties, and a program that reads one knows which it is paying for.
