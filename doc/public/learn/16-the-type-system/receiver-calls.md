# Receiver calls

`value.f(args)` is a **receiver call**: a call to a function declared with a
receiver, `self`, where the value fills `self` and the rest are the
arguments. It is how the book has written `"Dune".upper()`, `xs.size()` and
`o.unwrap_or(0)` since chapter 2, and this page is about what the dot does.

## Three spellings of one call

A function of a module is called through the module, as a receiver, or as
a pipeline step, and the three are one call:

```hql
[[3, 1, 2].sort(), [3, 1, 2] | sort, sort([3, 1, 2])]
```

```text
[[1, 2, 3], [1, 2, 3], [1, 2, 3]]
```

`xs.sort()` is `sort(xs)`, and `xs | sort` is `sort(xs)` too. They differ
in what they say to a reader. The dot is local evaluation, a function
applied to a value, and chains left to right. The pipe is dataflow, a step
in a pipeline the planner sees, and stays distinct from expression
evaluation. A program chains text functions with dots and pipes
collections through steps, and may write either.

```hql
"  Hello, World  ".trim().lower().split(", ").map(w => w.upper()).join(" / ")
```

```text
"HELLO / WORLD"
```

## How the name resolves

A receiver call is resolved by the value's type among the modules in scope.
The prelude's modules, `string`, `seq`, `math`, `option` and `result`
among them, are **receiver modules**, each declared for a type pattern:
`module string for String` makes each of its `self` functions a function of
every `String`. Two modules may declare one name over different types, and
the value's type picks: `"abc".contains("b")` is the string's,
`[1, 2].contains(2)` the collection's.

A module written `import string` is also reachable by name, so the one
function has a third spelling, `string.upper("hi")`:

```hql
import string
[string.upper("hi"), "hi".upper(), upper("hi")]
```

```text
["HI", "HI", "HI"]
```

A name no module in scope declares for the value's type is a name error,
with the nearest name offered:

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
that name ambiguous, a name error until it is written through its module.
A declaration the unit writes itself wins over every import's.
[Receiver modules](../05-modules/receiver-modules.md) declares them, and
[how names resolve](../05-modules/names.md) has the rules in full.

## A receiver module in a program

A program may open a receiver module for a type it declares, once the type
is in scope. The functions after the `module` line take `self` at the
pattern, and the type's values take them:

```hql
type Point { x : Int, y : Int }
module geometry for Point
fn norm(self) : Int = self.x * self.x
Point(x = 3, y = 4).norm()
```

```text
9
```

`self` is written untyped, since its type is the module's pattern. A
pattern may be generic, `module seq for Seq<T>`, and then every `Seq` takes
the functions.

## A trait's functions are receiver calls

A function of a trait is called on a value as a receiver function is, and
resolves to the implementation for the value's own type. Held at a wider
type, a union of two implementing types, the value still takes its own:

```hql
trait Named {
    fn name(self) : String
}
type Dog { tag : String }
type Cat { tag : String }
impl Named for Dog {
    fn name(self) : String = fmt("dog {}", self.tag)
}
impl Named for Cat {
    fn name(self) : String = fmt("cat {}", self.tag)
}
type Pet = union { Dog, Cat }
pets : List<Pet> = [Dog(tag = "rex"), Cat(tag = "tom")]
pets | map(p => match p { Dog d => d.name(), Cat c => c.name() })
```

```text
["dog rex", "cat tom"]
```

A trait's function has no module to call it through: `name(dog)` is a name
error, and the receiver form is the one spelling. [Traits](traits.md) is
about when a function belongs to a trait and when to a receiver module.

## A field is not a call

`p.x` without parentheses reads a field, and `p.x()` calls a function
named `x` on the value. The two never collide: a record declares its
fields, a module its functions, and the dot reads whichever the name is.
