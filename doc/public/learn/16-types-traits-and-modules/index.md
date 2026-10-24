# Types, traits and modules

The chapters so far query and transform data. This one is about the pieces a
program of its own needs: records, unions, results, receiver calls, traits and
modules. The [Mighty Types corpus](https://github.com/ewiger/hql/tree/main/examples/mighty-types)
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

`value.f(args)` calls a function declared with a receiver, `self`, with the
value as that receiver, resolved by the value's type among the modules in
scope. The prelude's modules, `string`,
`seq`, `math`, `option` and `result`, need no import, so text, sequences and
options have their functions at hand. `c.title` without parentheses is still
a field read.

```hql
"  Hello, World  ".trim().lower().split(", ").map(w => w.upper()).join(" / ")
```

```text
"HELLO / WORLD"
```

`string.trim(s)` is the same call written with its module, and `s | trim` the
same declaration as a pipeline step. Two modules may declare one name:
`"abc".contains("b")` is the string's and `[1, 2].contains(2)` the
collection's, told apart by the receiver. A function declared without
`self` takes no receiver: `range(1, 3)` and `1 | range(3)` call it, and
`1.range(3)` is refused.

## Traits and operators

A `trait` is a compile-time contract: the functions a type provides to
satisfy it, some with a default body. `impl Name for Type` provides it once
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

## Modules

`import a.b` makes one module visible under its last segment, `b`, and
`import a.b as c` under `c`. A standard module is reached the same way,
`import io.net` or `import regex`. A workspace module is an HQL file under a
**module root**, named by its position: `lib/mighty/geometry.hql` is
`mighty.geometry`, `lib/mighty/mod.hql` would be `mighty`. The roots come
from `hql.toml`, `[modules] roots = ["lib"]`, or from `--modules lib` on the
command line.

A module declares functions, types, traits and implementations, and may import
for itself:

````hql
// lib/mighty/geometry.hql
type Circle { radius : Float }
type Rect { width : Float, height : Float }

fn circle_area(c: Circle) : Float = 3.0f * c.radius * c.radius
fn rect_area(r: Rect) : Float = r.width * r.height
fn scale(r: Rect, k: Float) : Rect = Rect(width = r.width * k, height = r.height * k)
````

Its functions are reached as `geo.f(…)`; the types it declares are visible by
their names, since there is one lattice:

<!-- run: --modules examples/mighty-types/lib -->
```hql
import mighty.geometry as geo

geo.rect_area(geo.scale(Rect(width = 2.0f, height = 1.5f), 2.0f))
```

```text
12.0
```

Every module file opens with its name, checked against the file's place
under its root: `lib/mighty/text.hql` opens with `module mighty.text`. A module
that adds a pattern, `module mighty.text for String`, is a **receiver module**: its
functions take `self`, and once the module is imported they resolve as
`value.f()` for every value the pattern fits.

````hql
// lib/mighty/text.hql
module mighty.text for String

fn shout(self) : String = fmt("{}!", self.upper())
fn is_blank(self) : Bool = self.trim().length() == 0
````

<!-- run: --modules examples/mighty-types/lib -->
```hql
import mighty.text

["hello world".shout(), text.shout("hi"), str(text.is_blank("   "))]
```

```text
["HELLO WORLD!", "HI!", "true"]
```

`is_blank` is qualified because the prelude's `string` declares a receiver
`is_blank` too: two imported declarations of one name are an ambiguity, and
qualifying the call resolves it. A declaration the unit writes itself wins over
every import's, in every spelling.

Every module is an **import unit**: its imports, and the prelude, are visible
in it and nowhere else. A module is a closure: it sees no host binding, so
`vault`, `cards` and `stdin` belong to the program alone, and what a module
needs from a program reaches it as an argument. Inside a module a declaration
is public unless it is marked `private`; a private function, type, trait or
binding is visible only within its own module and is never reached by an
importer, by name, as a receiver call or as a step.

Imports sit at the head of a source unit, two imports may not share a local
name, and a module may not import itself through a cycle. The standard
library's own modules are written the same way, as contracts in `lib/std/`:
`lib/std/string/mod.hql` opens with `module string for String` and declares `fn trim(self) :
String` with no body, which the binary implements.
