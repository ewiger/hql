# 17. The type system

The chapters so far query and transform data with the types HQL provides.
This one is about the types a program declares for itself, and the
mechanisms that relate one type to another: records and unions, results,
subtypes, generics, receiver calls, operators and traits. The
[Mighty Types corpus](https://github.com/ewiger/hql/tree/main/examples/mighty-types)
has one program per topic.

## One relation symbol

Everything the type system says is said with `:`. Between a value and a
type it states what the value is, `pages : Int`. Between two types under
`type` it states that one narrows the other, `type Card : Doc`. Between a
type parameter and a type or trait it is a bound, `T : Orderable`. The
three readings are one relation: a value of the narrower type is usable
wherever the wider one is expected, and a bound asks for exactly that.

```hql
type Named { name : String }
type Person : Named { born : Int }

fn greet(n: Named) : String = fmt("hello, {}", n.name)

greet(Person(name = "Ada", born = 1815))
```

```text
"hello, Ada"
```

A `Person` is a `Named`, so `greet` takes one with no step and no check at
run time.

## Three mechanisms

Three mechanisms end with a value used at a stronger type than it started
with, and the chapter keeps them apart.

| Mechanism | Written | The question it answers |
| --- | --- | --- |
| Subtyping | `type Card : Doc` | Which types stand in for which? |
| Generics | `type Stack<T>` | What does this declaration leave unknown? |
| Refinement | `d \| is(Point)` | What is known about this one value? |

Subtyping relates whole types. Generics write a declaration once for a
family of types, and a bound is where the two meet: `T : Doc` restricts an
unknown by the subtype relation. Refinement is a statement about one
value, established by a test that can fail, and the value itself is never
changed. [Chapter 18](../18-advanced-data-typing/index.md) is refinement at
the boundary where data arrives.

## Capabilities are traits

What a type is and what can be done with it are two different statements.
`type Card : Doc` says a card is a document. That `+` adds two values of a
type, or that a generic function may call `area` on its parameter, is a
**capability**, stated by implementing a trait for the type. A trait is not
a type: nothing is held as a `Shape`, and a trait's name appears in a bound,
an implementation and another trait's declaration, nowhere else.

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

## Types in modules

A module declares types, traits and implementations as it declares
functions, and a public type is visible by its name to every unit that
imports the module. [Chapter 5](../05-modules/index.md) writes and imports
modules, and [visibility](../05-modules/visibility.md) keeps a type or
trait `private` to its module. An implementation belongs with its trait or
with its type: a program implements its own trait for any type and any trait
for its own type, and never a core trait for a built-in one.

## The pages

| Page | What it covers |
| --- | --- |
| [Records](records.md) | fields, the constructor, optional and nested fields, generic records, constructors a program declares |
| [Unions and match](unions-and-match.md) | a type that is one of several, `match` covering every case, guards, literal and type patterns |
| [Results](results.md) | `Result<T, E>`, `ok` and `err`, the ways out, and why there is no unwrap |
| [Subtypes and narrowing](subtypes.md) | `type A : B`, supertype sets, abstract types, widening by annotation, narrowing by test, conversion by construction |
| [Generics](generics.md) | type parameters, bounds, generic traits and the inference that fills them |
| [Receiver calls](receiver-calls.md) | `value.f(args)`, how the name resolves, and the three spellings of one call |
| [Operators](operators.md) | the core traits behind `+`, `==`, `<`, `xs[i]` and `xs[a:b]` |
| [Traits](traits.md) | required functions and default bodies, bounds, a trait extending a trait, and how a trait differs from inheritance |
