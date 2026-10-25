# Data: records and lists

The values JSON and YAML are made of — records of named fields, lists, and
scalar leaves — are one type in HQL, `Data`. This page does with `Data` what
`jq` does with JSON: build it, read into it, reshape it, and write it back out.

## A tree of values

A data literal is written in braces, with a key, a colon and a value per field.
Values may be scalars, lists, or other records:

```hql
{title: "Dune", author: {name: "Frank Herbert", born: 1920}, tags: ["desert", "spice"]}
```

```text
{author: {born: 1920, name: "Frank Herbert"}, tags: ["desert", "spice"], title: "Dune"}
```

A key that is not an identifier is quoted: `{"first name": "Ada"}`. The record
prints with its keys in order, so the same tree always prints the same way.

The colon is what makes braces a record. The empty record is `{:}`; `{}`
without a colon is the empty set, as `{1, 2}` is a set of two. So
`d : Data = {:}` holds an empty record, and `d : Data = {}` is a type error:
`{}` is a `Set<Never>`, which a `Set<Int>` binding accepts and a `Data` one
does not.

## Reading a field

A path reads into the tree, one key per step:

```hql
book = {title: "Dune", author: {name: "Frank Herbert", born: 1920}}
book.author.name
```

```text
"Frank Herbert"
```

A key the tree does not have is not an error. The key belonged to whoever wrote
the record, so nothing promised it, and reading it gives **absence**:

```hql
book = {title: "Dune"}
book.year
```

```text
none
```

The type says so. Reading a key of a `Data` record gives an `Option<Data>`: a
`some` holding a tree, or `none`. HQL has no null; a value that may be missing
is an `Option`, and the program says what absence means where it matters.

```hql
year = {title: "Dune"}.year
match year {
    some(_) => "dated",
    none => "undated",
}
```

```text
"undated"
```

## Asking what a leaf is

A leaf of a tree could be any scalar; the record did not say. Equality compares
through that, so `book.title == "Dune"` is an ordinary question:

```hql
{title: "Dune"}.title == "Dune"
```

```text
true
```

Arithmetic and ordering need to know the type. `is(T)` asks whether a value is
a `T`, and a `match` arm with a type, `some(Int n)`, both tests and reads it:

```hql
pages = {title: "Dune", pages: 412}.pages
match pages {
    some(Int n) => n * 2,
    _ => 0,
}
```

```text
824
```

## Lists of records

A list of records is a `List<Data>`. `map` applies a function to every element,
and `x => x.title` is a function written in place, a **lambda**:

```hql
books = [{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}, {title: "Leaves of Grass"}]
books | map(b => b.title)
```

```text
["Dune", "The Hobbit", "Leaves of Grass"]
```

`|` is a **pipe**: the value on its left is the input of the step on its right.
`filter` keeps the elements a predicate accepts:

```hql
books = [{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}, {title: "Leaves of Grass"}]
books
| filter(b => match b.pages { some(Int n) => n > 300, _ => false })
| map(b => b.title)
```

```text
["Dune", "The Hobbit"]
```

To add up the page counts, the leaves become numbers first. `somes` drops the
absent ones, `of_type(Int)` keeps the leaves that are integers and reads each as
the `Int` it holds, and `sum` adds them:

```hql
books = [{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}, {title: "Leaves of Grass"}]
books
| map(b => b.pages)
| somes
| of_type(Int)
| sum
```

```text
722
```

Each step is checked against the type the step before it produced. After
`of_type(Int)` the list is a list of `Int`, which is what `sum` takes; without
it the checker rejects `sum` over trees, before anything runs.

## Reshaping

A data literal can hold any expression, so `map` builds a new record from each
old one:

```hql
books = [{title: "Dune", author: "Frank Herbert"}, {title: "Emma", author: "Jane Austen"}]
books | map(b => {book: b.title, by: b.author})
```

```text
[{book: "Dune", by: "Frank Herbert"}, {book: "Emma", by: "Jane Austen"}]
```

`merge` combines two records. Every key of either is kept, and a key in both
takes the value from the right:

```hql
merge({title: "Dune", pages: 412}, {pages: 896, edition: "illustrated"})
```

```text
{edition: "illustrated", pages: 896, title: "Dune"}
```

## Writing it back out

A value is written in a **presentation format** by a step named for it. The
result is text of that format — `Json`, `Yaml`, `Csv`, `Markdown` — and
`hql` writes it as it is:

```hql
[{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}] | json
```

```text
[
  {
    "pages": 412,
    "title": "Dune"
  },
  {
    "pages": 310,
    "title": "The Hobbit"
  }
]
```

```hql
{title: "Dune", pages: 412, tags: ["desert", "spice"]} | yaml
```

```text
pages: 412
tags:
- desert
- spice
title: Dune
```

```hql
[{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}] | csv
```

```text
pages,title
412,Dune
310,The Hobbit
```

A list of records becomes a table when it is built into one. `Table` takes
the records as rows and `columns` names their fields in the order the table
writes them; each format then writes the table in its own syntax, here a pipe
table:

```hql
[{title: "Dune", pages: 412}, {title: "The Hobbit", pages: 310}]
| Table(columns = ["title", "pages"])
| markdown
```

```text
| title      | pages |
| ---------- | ----- |
| Dune       | 412   |
| The Hobbit | 310   |
```

[Chapter 8](../08-output.md) is about formats in full.

A program writes its data as literals, reads it from a vault's documents, as
[chapter 16](../16-a-vault.md) does, or reads a JSON, YAML or CSV file with
`source`, as [chapter 9](../09-io/source.md) does.

## What is next

How `Data` differs from a set and from a map, all three written in braces, is
[the next page](sets-data-and-maps.md). A `Data` list is one kind of
collection. [Chapter 3](../03-collections.md) meets
the others — lists, sets and maps of typed values — and the steps that work on
all of them.
