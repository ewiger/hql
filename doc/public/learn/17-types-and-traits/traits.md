# Traits

A **trait** is a capability contract over a type: a named set of functions
that an implementing type provides, each taking that type as `self`. A type
satisfies a trait through one **implementation**, `impl Trait for Type`. The
type checker verifies every implementation and every trait bound before the
program runs. A function of a trait is either **required**, a signature with
no body that every implementation supplies, or has a **default body**, which an implementation inherits unless it writes the
function itself.

## A trait and an implementation

`Shape` requires `area` and gives `describe` a default body. `impl Shape for
Rect` writes `area` alone:

<!-- define: shape -->
<!-- run: skip -->
```hql
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Rect { width : Float, height : Float }

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}
```

`describe` is a function of every `Rect` all the same, with the trait's body.
The programs below run with these declarations before them:

<!-- run: with shape -->
```hql
Rect(width = 2.0f, height = 3.0f).describe()
```

```text
"area 6.0"
```

## A default body calls the implementation

Inside a default body, `self.area()` resolves to the `area` of the type
`self` has. The body is written once, against the required functions, and
each type it runs over contributes its own:

<!-- run: with shape -->
```hql
type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

[Rect(width = 2.0f, height = 3.0f).describe(), Circle(radius = 1.0f).describe()]
```

```text
["area 6.0", "area 3.0"]
```

## Replacing a default

An implementation that writes a function the trait gives a default replaces
the default for its type, and leaves it in place for every other type:

<!-- run: with shape -->
```hql
type Square { side : Float }

impl Shape for Square {
    fn area(self) : Float = self.side * self.side
    fn describe(self) : String = fmt("a square of side {}", self.side)
}

[Rect(width = 2.0f, height = 3.0f).describe(), Square(side = 2.0f).describe()]
```

```text
["area 6.0", "a square of side 2.0"]
```

## What an implementation must supply

An implementation without one of the required functions is a type error at
its `impl` line, and the diagnostic names the function:

<!-- run: novault -->
```hql
trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

type Circle { radius : Float }

impl Shape for Circle {
    fn describe(self) : String = "a circle"
}

Circle(radius = 1.0f).describe()
```

```text
type error: `impl Shape for Circle` lacks `area`
 --> <expression>:8:1
  |
8 | impl Shape for Circle {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A trait has one implementation per type in a program. The checker rejects a
second:

<!-- run: novault -->
```hql
trait Named {
    fn name(self) : String
}

type Dog { tag : String }

impl Named for Dog {
    fn name(self) : String = self.tag
}

impl Named for Dog {
    fn name(self) : String = "a dog"
}

Dog(tag = "rex").name()
```

```text
type error: `Named` is already implemented for Dog, in this program, at byte 71: a trait has one implementation per type
  --> <expression>:11:1
   |
11 | impl Named for Dog {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Generic functions over a trait

A **bound**, `T : Shape`, restricts a type parameter to the types that
implement the trait. Inside the body `x.area()` resolves through the bound,
and every call is checked against it:

<!-- define: total -->
<!-- run: skip -->
```hql
type Circle { radius : Float }

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum
```

<!-- run: with shape,total -->
```hql
[total([Rect(width = 2.0f, height = 3.0f), Rect(width = 1.0f, height = 1.0f)]), total([Circle(radius = 1.0f)])]
```

```text
[7.0, 3.0]
```

A call with a type that has no implementation is a type error:

<!-- run: novault -->
```hql
trait Shape {
    fn area(self) : Float
}

type Line { length : Float }

fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum

total([Line(length = 2.0f)])
```

```text
type error: `total` needs T : Shape, and Line does not implement `Shape`
 --> <expression>:9:1
  |
9 | total([Line(length = 2.0f)])
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## A trait is not a type

A trait's name appears only where a contract is meant: in a bound, in an
`impl`, and in another trait's declaration. It is not a type, so no
parameter, binding or list is typed by one:

<!-- run: novault -->
```hql
trait Shape {
    fn area(self) : Float
}

fn doubled(s: Shape) : Float = s.area() * 2.0f

1
```

```text
name error: unknown type `Shape`
 --> <expression>:5:15
  |
5 | fn doubled(s: Shape) : Float = s.area() * 2.0f
  |               ^^^^^
```

A list that holds a `Rect` and a `Circle` together is typed by a union of the
two, and a `match` reaches each member at its own type, where its
implementation applies:

<!-- run: with shape,total -->
```hql
type Figure = union { Rect, Circle }

figures : List<Figure> = [Rect(width = 2.0f, height = 3.0f), Circle(radius = 1.0f)]

figures | map(f => match f {
    Rect r => r.describe(),
    Circle c => c.describe(),
})
```

```text
["area 6.0", "area 3.0"]
```

A later release names that union in the trait's declaration, `trait Shape as
Figure`, so that `List<Figure>` holds every implementing type and
`f.describe()` needs no `match`. The current release does not parse it yet.

## A trait extending a trait

A trait may **extend** others, `trait Greeter : Named`. The functions of the
traits it extends are its own, so its default bodies may call them, and an
implementation of it supplies them:

```hql
trait Named {
    fn name(self) : String
}

trait Greeter : Named {
    fn greet(self) : String = fmt("hello, {}", self.name())
}

type Dog { tag : String }

impl Greeter for Dog {
    fn name(self) : String = self.tag
}

Dog(tag = "rex").greet()
```

```text
"hello, rex"
```

Implementing `Greeter` does not implement `Named`. A bound `T : Named` takes
only the types with an `impl Named` of their own:

<!-- run: novault -->
```hql
trait Named {
    fn name(self) : String
}

trait Greeter : Named { }

type Dog { tag : String }

impl Greeter for Dog {
    fn name(self) : String = self.tag
}

fn names<T : Named>(xs: List<T>) : List<String> = xs | map(x => x.name())

names([Dog(tag = "rex")])
```

```text
type error: `names` needs T : Named, and Dog does not implement `Named`
  --> <expression>:15:1
   |
15 | names([Dog(tag = "rex")])
   | ^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Operators are traits

The operators resolve through the **core traits**, which are in scope
everywhere: `+` through `Add`, `-` through `Sub`, `*` through `Mul`, `/`
through `Div`, `%` through `Rem`, `==` through `Eq`, `xs[i]` through `Index`
and `xs[a:b]` through `Slice`. Implementing one gives a type its operator:

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

A type implementing `Ord` is ordered. Its `compare` decides `<`, `sort`, `min`
and `max`:

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

## Traits and inheritance

A default body is the part of class inheritance a trait has: code written
once against functions each type defines. The rest is absent.

| Class inheritance | A trait |
| --- | --- |
| a subclass inherits method bodies | an implementation inherits default bodies |
| a subclass inherits fields | a trait declares no fields |
| a subclass is a subtype of its parent | a trait is not a type; no value is held as a `Shape` |
| a method overrides along a chain of parents, reaching up with `super` | an implementation replaces a default, one level, with no `super` |
| a class extends one parent or several | a trait extends traits; a type implements each trait it needs, once |

Where one value is genuinely another, as a `Card` is a `Doc`, HQL says so
with an ordinary `:` between types, not with a trait.

## A receiver function without a trait

A receiver module adds functions to a type with no trait at all.
`module counting for Int` makes each `self` function after it a function of
every `Int`:

```hql
module counting for Int

fn twice(self) : Int = self * 2

[3.twice(), 21.twice()]
```

```text
[6, 42]
```

The functions reach only the source unit that declares the module or imports
it. Elsewhere the call is a name error, as `"hi".shout()` is without
`import mighty.text` ([receiver modules](../05-modules/receiver-modules.md)):

<!-- run: --modules examples/mighty-types/lib -->
```hql
"hi".shout()
```

```text
name error: String has no function `shout`; did you mean `sort`?
 --> <expression>:1:1
  |
1 | "hi".shout()
  | ^^^^^^^^^^^^
```

Two imported modules that declare one name over one type make a call by
that name ambiguous, and the call is a name error until it is qualified. A
declaration the unit writes itself wins over every import's.

A trait is the form for a function that generic code quantifies over or an
operator resolves through. A receiver module is the form for a function a
type gains that nothing quantifies over.

## A module that implements a trait

A later release joins the two forms: a receiver module lists traits after its pattern, and its
`self` functions are the implementation of those traits. The current release
does not parse the trait list yet:

<!-- run: skip -->
```hql
module shapes.circle for Circle : Shape

type Circle { radius : Float }

fn area(self) : Float = 3.0f * self.radius * self.radius
```

`module shapes.circle for Circle : Shape` means `impl Shape for Circle` with
the module's functions as the body: `describe` comes from the trait's
default, and `Circle(radius = 1.0f).describe()` is `"area 3.0"`.
