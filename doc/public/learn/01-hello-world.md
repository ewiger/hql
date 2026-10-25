# 1. Hello world

HQL is a statically typed expression and query language. A program is an
expression, or a few statements ending in one; `hql` checks its type, then
evaluates it, and writes the value. This chapter is about that loop.

## Hello, world

The smallest program that says hello is the text itself:

```hql
"Hello, world!"
```

```text
"Hello, world!"
```

A program is an expression, and its value is what `hql` writes. `fmt` writes
values into a template, one `{}` each:

```hql
fmt("Hello, {}!", "world")
```

```text
"Hello, world!"
```

## Running a program

`hql eval` takes a program as an argument:

```sh
hql eval '"Hello, world!"'
```

Every program in this book runs that way. Most of them run against the
tutorial vault, a small reading club in [`examples/tutorial/`](https://github.com/ewiger/hql/tree/main/examples/tutorial),
so the commands in later chapters start with `hql --vault examples/tutorial
eval`. The book shows each program in an `hql` block and what `hql` writes in
the block after it.

```hql
1 + 2
```

```text
3
```

## Values and their types

Every value has a type: `1 + 2` is an `Int`, `"Hello"` a `String`.
`hql check` writes a program's type without evaluating it:

```sh
$ hql check - <<< '7 / 2'
Int
```

[Chapter 2](02-data-types/index.md) meets the types one by one.

## Checked before it runs

A program is checked as a whole before anything is evaluated. A type error
names the expression it found and points at it:

```hql
1 + "two"
```

```text
type error: addition requires two Int, two Float or two Decimal operands
 --> <expression>:1:5
  |
1 | 1 + "two"
  |     ^^^^^
```

Nothing ran: there is no value, only the report, and the exit status is `1`.
The same holds for a name nobody bound:

```hql
pages + 1
```

```text
name error: `pages` is not bound
 --> <expression>:1:1
  |
1 | pages + 1
  | ^^^^^
```

## Bindings

`name = expression` binds a value to a name for the statements after it. The
program's value is its last expression.

```hql
pages = 280
chapters = 24
pages / chapters
```

```text
11
```

A binding may state its type, and the checker holds the value to it:

```hql
rating : Int = 5
rating
```

```text
5
```

## Choosing a value

`a given condition otherwise b` is `a` when the condition holds, and `b`
otherwise. It is an expression, so it has a type: both branches have one.

```hql
pages = 412
"long" given pages > 300 otherwise "short"
```

```text
"long"
```

`match` compares one value against patterns, in order, and the first that fits
gives the result:

```hql
status = "reading"
match status {
    "done" => "finished",
    "reading" => "on the nightstand",
    _ => "on the shelf",
}
```

```text
"on the nightstand"
```

The checker rejects a `match` that does not cover every case before it runs; `_` is the
pattern that fits anything.

## What is next

[Chapter 2](02-data-types/index.md) meets the data types: `Int`, `Float`,
`String` and `Bool`, tried in the REPL, and `Data`, the records and lists that
JSON and YAML are made of.
