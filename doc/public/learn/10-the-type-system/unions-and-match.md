# Unions and match

A **union** is a type whose value is exactly one of several alternatives.
`match` is how a value of one is taken apart, one arm per alternative, and
the checker requires every alternative to be covered.

## A union lists alternatives

`type Shape = union { Circle, Square }` declares that a `Shape` is a
`Circle` or a `Square`. A `match` over it binds each member at its own
type, and the arm sees that type's fields:

```hql
type Circle { radius : Float }
type Square { side : Float }
type Shape = union { Circle, Square }

fn area(s: Shape) : Float = match s {
    Circle c => 3.0f * c.radius * c.radius,
    Square q => q.side * q.side,
}

shapes : List<Shape> = [Circle(radius = 1.0f), Square(side = 2.0f)]
shapes | map(area)
```

```text
[3.0, 4.0]
```

The annotation on `shapes` holds the literal at the union. Without it, a
list of a `Circle` and a `Square` is a type error: the two meet nowhere on their
own, and the message says the elements have no common type. A union is
written with `=` and `union`, and that is the only way a type is one of
several; `type Id = Int` is not a declaration.

A `Circle` passes where a `Shape` is expected with no step, since it is a
member. Going the other way, from the union to a member, is a `match`, or
`is(Circle)` and `of_type(Circle)` over a collection, as
[subtypes and narrowing](subtypes.md) describes.

## A match covers every case

A `match` is checked for coverage. Leaving a member out is a type error, and
the message says how to complete it:

```hql
type Circle { radius : Float }
type Square { side : Float }
type Shape = union { Circle, Square }

fn area(s: Shape) : Float = match s {
    Circle c => 3.0f * c.radius * c.radius,
}

area(Circle(radius = 1.0f))
```

```text
check error[non-exhaustive-match]: this match does not cover every Shape: cover every member of the union with a type pattern, or add `_`
 --> <expression>:5:29
  |
5 | fn area(s: Shape) : Float = match s {
  |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

`_` fits anything. A match over an `Int` or a `String` has more values than
arms can name, so it needs one; a match over an `Option` needs `some` and
`none`, and one over a `Result` needs `ok` and `err`. This is what makes a
case safe to consume: a forgotten alternative is a type error, not a
silent default.

## Literal patterns and guards

A pattern may be a literal, a name that binds the value, or `_`. A **guard**,
`given` after the pattern, narrows an arm with a condition. The first arm
whose pattern fits and whose guard holds produces the result:

```hql
fn sign(n: Int) : String = match n {
    0 => "zero",
    k given k < 0 => "negative",
    _ => "positive",
}
[sign(-5), sign(0), sign(3)]
```

```text
["negative", "zero", "positive"]
```

A guarded arm does not count toward coverage, since its condition may
fail. Two guarded arms over an `Int` with no `_` are a type error, even where
their conditions together cover every number: the checker reads the
patterns, not the arithmetic.

## Type patterns over a tree

Over a `Data` value, whose leaf could hold any scalar, an arm written with a
type both tests and reads: `Int n` fits a leaf holding an integer and binds
it as an `Int`.

```hql
fn classify(x: Data) : String = match x {
    Int n => fmt("int {}", n),
    String s => fmt("text {}", s),
    _ => "other",
}
[classify(1), classify("hi"), classify(true)]
```

```text
["int 1", "text hi", "other"]
```

The same arm reads a tree as a declared record, `Book b`, when the tree has
the record's fields at their types, as [chapter 11](../11-advanced-data-typing/open-and-closed.md)
describes. `Data` is itself a union, declared in the standard library as a
scalar leaf, a sequence of trees or a string-keyed map of trees, which is
why whatever handles a `Data` may rely on there being three cases.

## Nested patterns

A pattern nests: `some(1)` fits a present value equal to one, `some(_)` any
present value.

```hql
match [1, 2, 3].first() {
    some(1) => "starts with one",
    some(_) => "starts otherwise",
    none => "empty",
}
```

```text
"starts with one"
```

## The short form

`given … otherwise` is a two-arm `match` on a `Bool`, read as a sentence:
this value, given a condition, otherwise that one.

```hql
[17, 42] | map(age => "adult" given age >= 18 otherwise "minor")
```

```text
["minor", "adult"]
```

Without `otherwise` the result is an `Option`: a value given a condition,
and nothing otherwise, is a value that may be absent. `"adult" given age >= 18`
over the same two ages is `[none, "adult"]`, and `otherwise` is the same
step as `unwrap_or` on that `Option`.
