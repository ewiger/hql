# 9. Streams that do not end

Every collection so far had an end: a size, a last element. A **stream** is a
succession of values whose production need not end — a log, a feed, the
natural numbers. HQL types the difference, so a program cannot ask an endless
stream a question that needs its end.

## A function that yields

A `fn` whose body is a block, in braces, produces its values with `yield`, in
the order each `yield` is reached. Its result type says whether production
ends: `Seq<T>` promises an end, `Stream<T>` does not. These are the natural
numbers:

<!-- define: naturals -->
```hql
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
naturals(1) | take(5)
```

```text
[1, 2, 3, 4, 5]
```

`for name in collection { … }` runs its block once per value. A block holds
`yield`, `for` and bindings, one per line, and is written only as the body of a
`fn`.

Calling `naturals(1)` evaluates nothing. Its value is a recipe for the
numbers, and `take(5)` asks for five of them and then stops asking — which is
what makes an endless producer usable.

## One value at a time

`map`, `filter` and `drop` take a stream and give a stream. Each value passes
through the whole pipeline before the next is produced, so only as many are
made as the end of the pipeline asks for:

<!-- run: with naturals -->
```hql
naturals(1)
| filter(n => n % 7 == 0)
| map(n => n * n)
| take(3)
```

```text
[49, 196, 441]
```

## Questions that need the end

`sum`, `size`, `sort`, `group` and the other whole-input steps need the last
value before they can answer. Over a stream there may be none, and the checker
says so before anything runs:

```hql
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
naturals(1) | sum
```

```text
type error: `sum` needs a finite input, and Stream<Int> may not end; bound it first with `take(n)`, or batch it with `collect(n)`
 --> <expression>:7:15
  |
7 | naturals(1) | sum
  |               ^^^
```

A stream becomes finite through `take(n)`, or is answered in pieces through
`collect(n)`, which gathers consecutive values into batches of `n`:

<!-- run: with naturals -->
```hql
naturals(1)
| collect(3)
| map(batch => batch | sum)
| take(3)
```

```text
[6, 15, 24]
```

`emit` is the inverse of `collect`: it passes on every member of every batch,
in order. `flat_map(f)` is `map(f) | emit`. Note the spaces in a list of lists:
`[[` opens a link to a card, so `[ [1, 2], [3] ]` is written with a space.

```hql
[ [1, 2], [3] ] | emit
```

```text
[1, 2, 3]
```

## A sequence over the vault

A generator that promises an end is a `Seq`. This one yields the title of each
book, in title order — a sequence the vault's cards produce on demand:

```hql
fn titles() : Seq<String> {
    for c in cards | filter(c => c.metadata.category == "book") | sort(by = c => c.title) {
        yield c.title
    }
}
titles() | take(3)
```

```text
[A Room of One's Own, Dune, Frankenstein]
```

`cards` itself is such a sequence: the vault's cards, produced as the walk of
the folder reaches each one.

## Writing a stream

Formatting an endless stream writes it piece by piece as it arrives: a line of
text, a JSON value per line, a CSV row. `hql` flushes each piece, and a reader
that stops reading ends the stream:

<!-- run: skip -->
```hql
naturals(1) | map(n => n * n) | json
```

```sh
$ hql eval '…naturals… | map(n => n * n) | json' | head -3
1
4
9
```

A diagram format describes a whole, so `| mermaid` over a stream is refused
until `take(n)` bounds it.

## What is next

[Chapter 10](10-output.md) is about every format a value can be written in, and
who chooses it.
