# Records

A `type` with fields is a **record**. It declares a shape: which fields a
value has, and the type of each. A record is a type of the program that
declares it, and nothing else is a `Point`.

## Declaring and building

A record is declared with its fields in braces, each a name, a colon and a
type, on one line separated by commas or one per line. Its **constructor**
is its name, taking the fields by name, and a dot reads a field:

```hql
type Point { x : Int, y : Int }

p = Point(x = 1, y = 2)
[p.x, p.y]
```

```text
[1, 2]
```

The fields may also be given in declaration order, `Point(1, 2)`. A value
prints as its constructor call, `Point(x = 1, y = 2)`, so what the book
shows is what builds it.

A record value is complete. Leaving a field out is a type error naming the
field, and a field at the wrong type is one naming the type:

```hql
type Point { x : Int, y : Int }
Point(x = 1)
```

```text
type error: `Point` needs `y`
 --> <expression>:2:1
  |
2 | Point(x = 1)
  | ^^^^^^^^^^^^
```

Reading a field the record does not declare is a name error, before the
program runs. A record has no hidden fields: what it declares is what a
value has, and all of it is readable.

## Nested records

A field may be of a record type, and a path reads through it:

```hql
type Point { x : Int, y : Int }
type Rectangle { top_left : Point, bottom_right : Point }

fn area(r: Rectangle) : Int = (r.bottom_right.x - r.top_left.x) * (r.bottom_right.y - r.top_left.y)

area(Rectangle(top_left = Point(x = 0, y = 0), bottom_right = Point(x = 4, y = 3)))
```

```text
12
```

## Optional fields

A field written `email? : String` may be absent. It reads as an
`Option<String>`, `none` where nothing was given, and it may be left out at
construction:

```hql
type Contact { name : String, email? : String }

[Contact(name = "Ada"), Contact(name = "Bob", email = "bob@example.com")] | map(c => c.email | unwrap_or("-"))
```

```text
["-", "bob@example.com"]
```

`Contact(name = "Ada")` prints as `Contact(name = "Ada", email = none)`.
Absence is a value of the field, and [chapter 2](../02-data-types/option.md)
has the ways out of an `Option`.

## Equality

Two records of one type are equal when their fields are, field by field.
Nothing is declared for it:

```hql
type Point { x : Int, y : Int }
[Point(x = 1, y = 2) == Point(x = 1, y = 2), Point(x = 1, y = 2) == Point(x = 2, y = 1)]
```

```text
[true, false]
```

A type that wants another equality implements `Eq`, as
[operators](operators.md) shows.

## Generic records

A record may take type parameters, and takes them from the values it is
built from: `Stack(items = [1, 2])` is a `Stack<Int>`, and
`Stack(items = ["a"])` a `Stack<String>`. A function over it names the same
parameter:

```hql
type Stack<T> { items : List<T> }

fn push<T>(s: Stack<T>, x: T) : Stack<T> = Stack(items = concat(s.items, [x]))
fn peek<T>(s: Stack<T>) : Option<T> = s.items.last()

s = Stack(items = [1, 2]) | push(3)
peek(s)
```

```text
3
```

Pushing `"three"` onto a `Stack<Int>` is a type error: within one use a parameter
names one type in every position it occurs. The prelude's `Pair<A, B>` is a
generic record, the element `zip` and `enumerate` produce, and
`Pair(first = p.second, second = p.first)` swaps one.

A parameter may carry a bound, written after the fields:
`type Box<T> { value : T } where T : Orderable`. A `Box` of a set is then a
type error at construction, since a set has no order. [Generics](generics.md)
is about parameters and bounds in full.

## A record under another

A record declared under another record, `type Person : Named { born : Int }`,
has the fields of both. A `Person` is a `Named`, so it is taken wherever a
`Named` is expected, as the [chapter's first program](index.md) showed, and
a contract's rules are inherited the same way
([type contracts](../11-advanced-data-typing/type-contracts.md)).

## Constructors a program declares

Beside the constructor its fields give it, a type may declare constructors
of its own: a function named after the type, in the unit that declares the
type, returning it. `Temp` below is built from a temperature in Celsius as
well as from its field:

```hql
type Temp { kelvin : Float }

fn Temp(celsius: Float) : Temp = Temp(kelvin = celsius + 273.0f)

[Temp(1.0f).kelvin, (2.0f | Temp).kelvin, Temp(kelvin = 5.0f).kelvin]
```

```text
[274.0, 275.0, 5.0]
```

A constructor is an operation like any other, so it has two forms: called,
`Temp(1.0f)`, and as a pipeline step, `2.0f | Temp`, where the piped value
is its first argument. It is also a function value: `xs | map(Temp)` builds
one per element. A type may have several constructors, told apart by the
type of their first parameter.

A function in `UpperCamelCase` is a constructor and nothing else. One named
after no declared type is a type error, and the message says to name it in
`snake_case`. The standard constructors follow the same rule: `List(xs)`,
`Set(xs)` and `Map(keys, values)` are declared beside their types, and
[subtypes and narrowing](subtypes.md) uses them as the conversion between
collections.
