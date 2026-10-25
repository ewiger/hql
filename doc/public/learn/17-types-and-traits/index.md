# 17. Types and traits

The chapters so far query and transform data. This one is about the types a
program declares for itself: records, unions, results, and traits with the
operators that resolve through them. The [Mighty Types corpus](https://github.com/ewiger/hql/tree/main/examples/mighty-types)
has one program per topic.

## Records

A `type` with fields is a record. Its constructor is its name, taking the
fields by name or in declaration order, and a dot reads a field. A record is
a type of the program that declares it, and nothing else is a `Point`.

```hql
type Point { x : Int, y : Int }

fn shift(p: Point, dx: Int) : Point = Point(x = p.x + dx, y = p.y)

shift(Point(x = 1, y = 2), 10)
```

```text
Point(x = 11, y = 2)
```

A record may be generic, and takes its parameters from the values it is built
from: `Stack(items = [1, 2])` is a `Stack<Int>`. A field written `email? :
String` is optional, reads as `Option<String>`, and may be left out at
construction.

## Unions, and a match that covers them

A union lists alternatives. A `match` binds each member at its own type and
must cover every one. The annotation on `shapes` holds the literal at the
union, since a `Circle` and a `Square` meet nowhere on their own.

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

## Results

`Result<T, E>` is a computation that may have failed: `ok(v)` or `err(e)`.
`ok(p)` and `err(p)` match one, and a match covers both. The ways out mirror
`Option`'s, `unwrap_or`, `map_err`, `and_then`, and there is no unwrap.

```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err(fmt("not a number: {}", text)),
}

["4", "x"] | map(parse)
```

```text
[ok(4), err("not a number: x")]
```

## Receiver calls

`value.f(args)` calls a function declared with a receiver, `self`, resolved by
the value's type among the modules in scope. [Receiver modules](../05-modules/receiver-modules.md)
declares them, and the prelude's `string`, `seq`, `math`, `option` and
`result` need no import:

```hql
"  Hello, World  ".trim().lower().split(", ").map(w => w.upper()).join(" / ")
```

```text
"HELLO / WORLD"
```

## Traits and operators

A `trait` is a capability contract: the functions a type provides to
satisfy it, some with a default body, verified by the type checker. `impl Name for Type` provides it once
per trait and type, and the operators resolve through the core traits: `+`
through `Add`, `==` through `Eq`, and a type implementing `Ord` sorts.

```hql
type Vec2 { x : Int, y : Int }

impl Add for Vec2 {
    fn add(self, other: Vec2) : Vec2 = Vec2(x = self.x + other.x, y = self.y + other.y)
}

Vec2(x = 1, y = 2) + Vec2(x = 3, y = 4)
```

```text
Vec2(x = 4, y = 6)
```

[Traits](traits.md) has a page of its own: required functions and default
bodies, bounds, a trait extending a trait, and how a trait differs from
class inheritance.

## Types in modules

A module declares types and traits as it declares functions, and a public
type is visible by its name to every unit that imports the module.
[Chapter 5](../05-modules/index.md) writes and imports modules, and
[visibility](../05-modules/visibility.md) keeps a type or trait `private` to
its module.

## What is next

A `Result` is one of the two ways a failure appears.
[Chapter 18](../18-errors/index.md) is the whole error model: failures kept as
data, and the diagnostics a run writes when a phase cannot do what the
program asked.
