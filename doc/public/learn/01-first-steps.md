# 1. First steps

HQL is a statically typed expression and query language. A program is an
expression, or a few statements ending in one; `hql` checks its type, then
evaluates it, and writes the value. This chapter is about that loop.

## Running a program

`hql eval` takes a program as an argument:

```sh
hql eval '1 + 2'
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

The scalar types are `Int`, `Float`, `String` and `Bool`. Arithmetic is defined
on two `Int` or two `Float` operands; `Int` division truncates.

```hql
7 / 2
```

```text
3
```

```hql
7.0 / 2.0
```

```text
3.5
```

```hql
"Frankenstein" == "Frankenstein" and 3 < 4
```

```text
true
```

`hql check` writes a program's type without evaluating it:

```sh
$ hql check - <<< '7 / 2'
Int
```

## Checked before it runs

A program is checked as a whole before anything is evaluated. A type error
names the expression it found and points at it:

```hql
1 + "two"
```

```text
type error: addition requires two Int or two Float operands
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

A `match` that does not cover every case is refused before it runs; `_` is the
pattern that fits anything.

## What is next

Scalars are only the leaves. [Chapter 2](02-data.md) builds trees of them —
the records and lists that JSON and YAML are made of — and asks them
questions.
