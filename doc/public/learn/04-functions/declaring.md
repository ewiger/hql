# Declaring a function

`fn` declares a named function: a name, typed parameters, a result type and a
body. `fn days(pages: Int, per_day: Int) : Int` gives `days` the type
`(Int, Int) -> Int`.

```hql
fn days(pages: Int, per_day: Int) : Int = (pages + per_day - 1) / per_day
[days(412, 40), days(280, 40)]
```

```text
[11, 7]
```

Every parameter has a type and the result type is written. Each call is
checked against them before the program runs:

```hql
fn days(pages: Int, per_day: Int) : Int = (pages + per_day - 1) / per_day
days("412", 40)
```

```text
type error: this argument is String, and parameter `pages` takes Int
 --> <expression>:2:6
  |
2 | days("412", 40)
  |      ^^^^^
```

The body after `=` is one expression. A body may also be a block of named
steps, in braces; [Function bodies](bodies.md) covers both forms.

## A function by its name

Naming a declared function where a function is expected passes it as a
value, `xs | map(f)`. Its name is bound before its body is checked, so it may
call itself:

```hql
fn factorial(n: Int) : Int = 1 given n <= 1 otherwise n * factorial(n - 1)
[1, 2, 3, 4, 5] | map(factorial)
```

```text
[1, 2, 6, 24, 120]
```

## Functions taken and returned

A parameter may have a function type, and so may the result. A returned
function keeps the values it captured:

```hql
fn adder(k: Int) : Int -> Int = n => n + k
fn twice(f: Int -> Int, x: Int) : Int = f(f(x))
twice(adder(3), 30)
```

```text
36
```

## Variadic parameters

A last parameter written `...T` collects any number of trailing arguments into
a list:

```hql
fn total(xs: ...Int) : Int = xs | sum
[total(), total(1, 2), total(1, 2, 3, 4)]
```

```text
[0, 3, 10]
```

## A result that may be missing

`Option<T>` is the result type of a function whose value may be absent.
`some(x)` and `none` build one, `match` takes one apart, and `somes` keeps the
present values of a collection:

```hql
fn rating_of(r: Int) : Option<Int> = some(r) given r > 0 otherwise none
[5, 0, 4] | map(rating_of) | somes
```

```text
[5, 4]
```

[Option](../02-data-types/option.md) has the rest of `Option`'s operations.
