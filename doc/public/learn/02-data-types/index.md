# 2. Data types

Every value in HQL has a type, and the checker knows it before the program
runs. This chapter meets the types values are built from: the scalars `Int`,
`Float`, `Decimal`, `String` and `Bool`, and `Data`, the tree JSON and YAML
are made of.
Each has a page of its own; this one tries them in the REPL first.

## Trying values in the REPL

`hql repl` reads one input at a time, evaluates it and prints the value. A
binding is kept for the inputs after it, and `:type` asks for a type without
evaluating anything:

```text
$ hql repl
hql 0.5.0. `:help` for commands, `:quit` to leave.
hql> pages = 412
()
hql> pages / 3
137
hql> pages % 3
1
hql> :type pages
Int
hql> title = "Dune"
()
hql> title.upper()
"DUNE"
hql> :type title
String
hql> fmt("{} has {} pages", title, pages)
"Dune has 412 pages"
hql> :quit
```

`412` is an `Int`, so `/` divides whole numbers and drops the remainder, and
`%` gives the remainder. `"Dune"` is a `String`, and `title.upper()` calls a
function of `String` on it. `fmt` writes values into text, one `{}` each.

The same programs run with `hql eval`, as every program in this book does:

```hql
pages = 412
title = "Dune"
fmt("{} has {} pages", title, pages)
```

```text
"Dune has 412 pages"
```

## A type is checked, not guessed

Each operation says which types it takes. An `Int` and a `String` do not add,
and the checker rejects the program before it runs:

```hql
412 + "pages"
```

```text
type error: addition requires two Int, two Float or two Decimal operands
 --> <expression>:1:7
  |
1 | 412 + "pages"
  |       ^^^^^^^
```

There is no implicit conversion between types, except that an `Int` beside a
`Decimal` widens to one. `str` writes a value as text, and `float` turns an
`Int` into a `Float`:

```hql
412.str()
```

```text
"412"
```

`hql check` prints a program's type without evaluating it:

```sh
$ hql check - <<< '412 / 3'
Int
```

## The types, one page each

| Type | What it holds | Page |
| --- | --- | --- |
| `Int` | a whole number, 64 bits | [Int](int.md) |
| `Float` | a binary floating-point number, written `2.5f` | [Float](float.md) |
| `Decimal` | an exact decimal number, written `2.50` | [Decimal](decimal.md) |
| `String` | text | [String](string.md) |
| `Bool` | `true` or `false` | [Bool](bool.md) |
| `Data` | a tree of records, lists and scalars | [Data](data.md) |
| `Option<T>` | a `T`, or nothing | [Option](option.md) |

Braces write three different things — a set, a `Data` record and, where one
is declared, a map — and [sets, Data and maps](sets-data-and-maps.md) tells
them apart. Lists, sets and maps as collections, and the steps they share, are
[chapter 3](../03-collections.md).
