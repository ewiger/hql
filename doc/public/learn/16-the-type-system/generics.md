# Generics

A declaration written once for every type names a **type parameter**, a
capital letter in angle brackets. Each use fills it: `Stack<Int>`,
`first([1, 2])`. Within one use a parameter names one type in every
position it occurs, and the declaration is checked once for all of them.

## Parameters and bounds

[Generic functions](../04-functions/generics.md) introduced the parameter
and its **bound**. Inside a body, `T` is a type of its own, and the body may
rely only on what the parameter promises. `T : Orderable` promises an
order, so the body may take a minimum, and each call is checked against the
bound:

```hql
fn shortest<T : Orderable>(xs: List<T>) : Option<T> = xs | min
[shortest([3, 1, 2]), shortest(["b", "a"]) | map(s => s.length())]
```

```text
[1, 1]
```

A bound is a type, as `Orderable` is, or a trait, as `Shape` is in
[traits](traits.md). Both are written with `:`, since both restrict the
unknown by the one relation the language has. A record's parameter takes a
bound after its fields, `type Box<T> { value : T } where T : Orderable`, and
a value that breaks it is a type error at construction.

## A generic trait

A trait names another type only through a parameter. `Index<T>` in the
standard library is the capability behind `xs[i]`, and `T` is the element
type its `at` returns. An implementation gives the trait its argument, and
takes parameters of its own to bind the implementing type's:

```hql
type Ring<T> { items : List<T> }

impl<T> Index<T> for Ring<T> {
    fn at(self, position: Int) : Option<T> = self.items[position % size(self.items)]
}

birds = Ring(items = ["raven", "robin", "wren"])
[birds[0], birds[4], birds.at(5)]
```

```text
["raven", "robin", "wren"]
```

`<` after `impl` opens the implementation's parameters, and `<` after the
trait's name its arguments. Each parameter of the implementation must
appear in the implementing type or in the trait's arguments; one that
appears in neither is a type error. A trait named with the wrong number of
arguments is one too: `impl Index for Box<Int>` says `Index` takes one
type argument, not none.

## Inference fills what a call leaves open

A bound may name a generic trait with the function's own parameters as its
arguments, `E : Index<T>`. A call fills `E` from its argument, and `T` from
the one implementation of `Index` that type has:

```hql
type Countdown { from : Int }

impl Index<Int> for Countdown {
    fn at(self, position: Int) : Option<Int> = match position {
        p given p >= 0 and p <= self.from => some(self.from - p),
        _ => none,
    }
}

fn first_of<T, E : Index<T>>(xs: E) : Option<T> = xs[0]

[first_of([10, 20]), first_of(Countdown(from = 3))]
```

```text
[10, 3]
```

A `List<Int>` takes the standard library's implementation for sequences,
which gives `T = Int`; a `Countdown` takes its own, `Index<Int>`. A type
with no implementation is a type error at the call, and the message names
the bound: `first_of(1)` says an `Int` does not implement `Index<T>`. There are
no associated types. A type that the implementing type decides, as the
element type is, is a trait parameter that inference fills.

## One trait at several arguments

A type may implement one trait at several arguments, and a call's argument
types choose between them:

```hql
trait Scale<F> {
    fn scale(self, by: F) : Self
}

type Vec2 { x : Float, y : Float }

impl Scale<Float> for Vec2 {
    fn scale(self, by: Float) : Vec2 = Vec2(x = self.x * by, y = self.y * by)
}

impl Scale<Int> for Vec2 {
    fn scale(self, by: Int) : Vec2 = self.scale(float(by))
}

v = Vec2(x = 1.5f, y = 2.0f)
[v.scale(2).x, v.scale(0.5f).x]
```

```text
[3.0, 0.75]
```

`Self` in the trait is the implementing type, `Vec2` in both
implementations. A generic body reaches an implementation through its
receiver alone, so a bound over a trait with functions, `T : Scale<F>`,
needs a type that implements the trait once; a type implementing it at
several arguments is a type error there, since the body could not tell
them apart.

## One implementation per trait and type

An implementation is identified by its trait, the trait's arguments and the
implementing type, and two that could apply to one type at one argument
list are a type error, whichever is generic:

```hql
type Box<T> { value : T }
impl<T> Index<T> for Box<T> {
    fn at(self, position: Int) : Option<T> = some(self.value)
}
impl Index<Int> for Box<Int> {
    fn at(self, position: Int) : Option<Int> = some(self.value)
}
Box(value = 1)[0]
```

```text
type error: `impl Index<Int> for Box<Int>` overlaps `impl Index<T> for Box<T>`, in this program, at byte 26: one type at one set of arguments has one implementation of a trait
 --> <expression>:5:1
  |
5 | impl Index<Int> for Box<Int> {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The rule holds across the standard library, the imported modules and the
program together, and import order never chooses. An implementation for a
type applies to the types narrowing it, and where two apply the narrower
type's is the one, so a `List` takes the implementation for `Seq`. No
implementation specializes another.

## Variance

Values are never changed in place, so a parameter a type only produces is
covariant: a `List<Person>` is a `List<Named>`, and an `Option<Person>` an
`Option<Named>`, as the [previous page](subtypes.md) used. A parameter a
type consumes is not: a `Map` is invariant in its keys, since lookup
accepts them, and a function type `T -> U` is contravariant in `T` and
covariant in `U`.
