# Types, traits and modules

The chapters so far query and transform data. This one is about the pieces a
program of its own needs: records, unions, results, receiver calls, traits and
modules. The [Mighty Types corpus](https://github.com/ewiger/hql/tree/main/examples/mighty-types)
has one program per topic; the design is
[HQL-0024](https://github.com/ewiger/hql/tree/main/doc/proposals/HQL-0024).

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
    Circle c => 3.0 * c.radius * c.radius,
    Square q => q.side * q.side,
}

shapes : List<Shape> = [Circle(radius = 1.0), Square(side = 2.0)]

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
per trait and type, and a call on a value dispatches on the value's own type.
The operators resolve through the core traits: `+` through `Add`, `==`
through `Eq`, and a type implementing `Ord` sorts.

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

A bound, `T : Named`, is checked at every call, and inside the body `x.name()`
resolves through it:

```hql
trait Named {
    fn name(self) : String
}

type Dog { tag : String }
type Rock { weight : Int }

impl Named for Dog {
    fn name(self) : String = self.tag
}

impl Named for Rock {
    fn name(self) : String = fmt("rock of {}", self.weight)
}

fn names<T : Named>(xs: List<T>) : List<String> = xs | map(x => x.name())

[names([Dog(tag = "rex")]), names([Rock(weight = 3)])]
```

```text
[["rex"], ["rock of 3"]]
```

`impl Ord for Version` supplies `compare`, and with it `<`, `sort`, `min` and
`max` take the type:

```hql
type Version { major : Int, minor : Int }

impl Ord for Version {
    fn compare(self, other: Version) : Ordering =
        compare(self.major * 100 + self.minor, other.major * 100 + other.minor)
}

[Version(major = 1, minor = 10), Version(major = 1, minor = 2)] | sort | map(v => v.minor)
```

```text
[2, 10]
```

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

fn circle_area(c: Circle) : Float = 3.0 * c.radius * c.radius
fn rect_area(r: Rect) : Float = r.width * r.height
fn scale(r: Rect, k: Float) : Rect = Rect(width = r.width * k, height = r.height * k)
````

Its functions are reached as `geo.f(…)`; the types it declares are visible by
their names, since there is one lattice:

<!-- run: --modules examples/mighty-types/lib -->
```hql
import mighty.geometry as geo

geo.rect_area(geo.scale(Rect(width = 2.0, height = 1.5), 2.0))
```

```text
12.0
```

A module that opens with `module for String` is a **receiver module**: its
functions take `self`, and once the module is imported they resolve as
`value.f()` for every value the pattern fits.

````hql
// lib/mighty/text.hql
module for String

fn shout(self) : String = fmt("{}!", self.upper())
fn is_blank(self) : Bool = self.trim().length() == 0
````

<!-- run: --modules examples/mighty-types/lib -->
```hql
import mighty.text

["hello world".shout(), text.shout("hi"), str("   ".is_blank())]
```

```text
["HELLO WORLD!", "HI!", "true"]
```

Imports sit at the head of a source unit, two imports may not share a local
name, and a module may not import itself through a cycle. The standard
library's own modules are written the same way, as contracts in `std/`:
`std/string.hql` opens with `module for String` and declares `fn trim(self) :
String` with no body, which the binary implements.
