# 4. Functions

A function is a value of a function type. It can be bound to a name, passed to
a step, returned from another function, and written once for every type it
works on. This page meets lambdas and function types; the pages after it
declare functions with `fn`.

## Lambdas and their types

`n => n * 2` is a lambda. Written as an argument, it takes its parameter's
type from the step it is passed to. Written on its own, it states the type:

```hql
double = (n: Int) => n * 2
[412, 280] | map(double)
```

```text
[824, 560]
```

A function's type is written with an arrow: `Int -> Int`, `(Int, Int) -> Int`.

<!-- run: check -->
```hql
double = (n: Int) => n * 2
double
```

```text
Int -> Int
```

A lambda's body is one expression. A function whose value takes several
steps is declared with `fn`, whose body may be a block of them.

## Calling a function

`f(x)` calls a function. Piping into one calls it too: `x | f` is `f(x)`, and
`x | f(y)` is `f(x, y)`, the piped value filling the first parameter.

```hql
double = (n: Int) => n * 2
280 | double
```

```text
560
```

## The pages of this chapter

| Page | What it covers |
| --- | --- |
| [Declaring a function](declaring.md) | `fn`, typed parameters and a result, recursion, functions taken and returned, variadic parameters |
| [Function bodies](bodies.md) | the two body forms, `= expr` and the block `{ … }`, and the rules a block follows |
| [Generic functions](generics.md) | type parameters, and the bounds a call is checked against |
| [Aliases and holes](aliases-and-holes.md) | `:=` names a pipeline fragment, and `_` leaves an argument open |

## What is next

A program's functions sit beside its expressions. [Chapter 5](../05-modules/index.md)
puts functions into modules of their own: files that a program imports, with
public and private declarations.
