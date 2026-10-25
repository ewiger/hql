# Option: a value that may be missing

The reading club gives a book its rating when the book is done. A book still
being read has no rating yet, so the club's notes have a field some records
carry and others do not:

<!-- define: books -->
```hql
books = [
    {title: "Frankenstein", status: "done", rating: 5},
    {title: "Dune", status: "reading"},
    {title: "A Room of One's Own", status: "done", rating: 4},
]
books | map(b => b.rating)
```

```text
[5, none, 4]
```

HQL has no null. A value that may be missing is an `Option<T>`: `some(x)`,
which holds an `x`, or `none`, which holds nothing. A present value prints as
the value it holds, and an absent one as `none`.

## The type says so

Reading a field of a `Data` record gives an `Option<Data>`, whichever record it
reads, because the type is known before any record is:

<!-- run: check -->
```hql
{title: "Dune"}.rating
```

```text
Option<Data>
```

An `Option` is not the value it may hold. Adding to a rating that may be
missing is a type error, reported before the program runs:

```hql
rating = {title: "Frankenstein", rating: 5}.rating
rating + 1
```

```text
type error: addition requires two Int, two Float or two Decimal operands, or two values of a type implementing `Add`, not Option<Data>
 --> <expression>:2:1
  |
2 | rating + 1
  | ^^^^^^
```

The program says what a missing rating means, and the ways to say it follow.

## A value in place of none

`unwrap_or(default)` gives the value held, or `default` where there is none:

<!-- run: with books -->
```hql
books | map(b => b.rating.unwrap_or(0))
```

```text
[5, 0, 4]
```

There is no `unwrap` without a default. Every way out of an `Option` states
what absence means.

## One answer for each case

`match` takes an `Option` apart. The `some(r)` arm binds the value held to `r`,
the `none` arm covers the rest, and the checker rejects a `match` that leaves
a case uncovered:

<!-- run: with books -->
```hql
books | map(b => {
    book: b.title,
    verdict: match b.rating {
        some(r) => fmt("{} stars", r),
        none => "not rated yet",
    },
})
```

```text
[{book: "Frankenstein", verdict: "5 stars"}, {book: "Dune", verdict: "not rated yet"}, {book: "A Room of One's Own", verdict: "4 stars"}]
```

`is_some` and `is_none` ask the question as a `Bool`, for a filter or a
`given`:

<!-- run: with books -->
```hql
books | filter(b => b.rating.is_none()) | map(b => b.title)
```

```text
["Dune"]
```

## Only the values that are there

`somes` keeps the present values of a collection of options and drops the
absent ones. The club's ratings so far are the ratings of the books it has
finished:

<!-- run: with books -->
```hql
books | map(b => b.rating) | somes
```

```text
[5, 4]
```

## Comparing with a missing value

Equality compares through an `Option`: `some(5) == 5` is `true`, and `none` is
equal to no value. A book with no rating is not a five-star book:

<!-- run: with books -->
```hql
books | filter(b => b.rating == 5) | map(b => b.title)
```

```text
["Frankenstein"]
```

## Options from other places

A record field is one source of options; a step that may have no answer is
another. `to_int` reads a `String` as an `Int`, and gives `none` where the
text is not a number:

```hql
["412", "four", "310"] | map(s => s.to_int())
```

```text
[412, none, 310]
```

A step applied to an `Option` is applied to the value held. `map` gives the
result in a `some`, or `none` where there was nothing to apply it to; `filter`
keeps a value that passes a test; and `alternative` gives a second option
where the first is `none`:

```hql
[
    "412".to_int().map(n => n / 2),
    "four".to_int().map(n => n / 2),
    "1965".to_int().filter(y => y > 1900),
    "four".to_int().alternative("0".to_int()),
]
```

```text
[206, none, 1965, 0]
```

`max` over an empty list, a position past the end of a string and a link to a
card nobody has written are each an `Option` too, and each is handled the same
way.

A missing value is not a failure. A computation that can fail gives a
`Result`, `ok` or `err`, and [chapter 16](../16-the-type-system/results.md)
meets it.
