# 6. SQL-like Queries

SQL asks a table questions: which rows, in what order, how many, grouped by
what. HQL asks the same questions with pipeline steps. This chapter works
through them over one table — the reading club's shelf, written as a list of
records.

## The table

<!-- define: shelf -->
```hql
shelf = [
    {title: "Frankenstein", author: "Mary Shelley", year: 1818, genre: "novel", pages: 280, status: "done", rating: 5},
    {title: "Dune", author: "Frank Herbert", year: 1965, genre: "novel", pages: 412, status: "reading"},
    {title: "The Hobbit", author: "J. R. R. Tolkien", year: 1937, genre: "novel", pages: 310, status: "todo"},
    {title: "Leaves of Grass", author: "Walt Whitman", year: 1855, genre: "poetry", pages: 145, status: "todo"},
    {title: "A Room of One's Own", author: "Virginia Woolf", year: 1929, genre: "essay", pages: 112, status: "done", rating: 4},
    {title: "The Common Reader", author: "Virginia Woolf", year: 1925, genre: "essay", pages: 304, status: "todo"},
    {title: "Mrs Dalloway", author: "Virginia Woolf", year: 1925, genre: "novel", pages: 194, status: "reading"},
]
shelf | size
```

```text
7
```

Every program in this chapter begins with these lines; the book shows them
once.

## Select and where

`map` is SQL's `SELECT`: it says what to keep of each row. `filter` is
`WHERE`:

<!-- run: with shelf -->
```hql
shelf
| filter(b => b.status == "todo")
| map(b => b.title)
```

```text
["The Hobbit", "Leaves of Grass", "The Common Reader"]
```

<!-- run: with shelf -->
```hql
shelf
| filter(b => b.author == "Virginia Woolf")
| map(b => {title: b.title, year: b.year})
```

```text
[{title: "A Room of One's Own", year: 1929}, {title: "The Common Reader", year: 1925}, {title: "Mrs Dalloway", year: 1925}]
```

## Typed views of a record

`ORDER BY pages` needs pages to be numbers, and a record's leaves have no type
of their own. A small function gives a record a **typed view** — one field, as
the type the program means it to be, with what to do when it is missing:

<!-- define: views -->
<!-- run: skip -->
```hql
fn title(b: Data) : String = match b.title { some(String t) => t, _ => "" }
fn pages(b: Data) : Int = match b.pages { some(Int n) => n, _ => 0 }
fn year(b: Data) : Int = match b.year { some(Int n) => n, _ => 0 }
```

From here on, the programs begin with the table and these views.

## Order by and limit

`sort(by = key)` is `ORDER BY`, and `take(n)` is `LIMIT`. A comparison orders
the other way:

<!-- run: with shelf,views -->
```hql
shelf
| sort(by = year)
| take(3)
| map(title)
```

```text
["Frankenstein", "Leaves of Grass", "The Common Reader"]
```

<!-- run: with shelf,views -->
```hql
shelf
| sort((a, b) => compare(pages(b), pages(a)))
| take(2)
| map(b => {title: title(b), pages: pages(b)})
```

```text
[{pages: 412, title: "Dune"}, {pages: 310, title: "The Hobbit"}]
```

## Distinct

A set keeps each value once, so `SELECT DISTINCT` is the `Set` constructor:

<!-- run: with shelf -->
```hql
shelf | map(b => b.genre) | Set
```

```text
{"novel", "poetry", "essay"}
```

## Aggregates

`size` counts rows. `sum`, `average`, `min` and `max` work over numbers, so the
pages are read through their view first:

<!-- run: with shelf,views -->
```hql
lengths = shelf | map(pages)
{books: lengths | size, pages: lengths | sum, longest: lengths | max, average: lengths | average}
```

```text
{average: 251.0, books: 7, longest: 412, pages: 1757}
```

Every aggregate is a **fold**: `reduce` with a starting value and a way to
combine two values. `sum` is `reduce(0, +)`. `max` is `reduce(max)`, which
starts from the first value and so is an `Option`: an empty list has none.

<!-- run: with shelf,views -->
```hql
lengths = shelf | map(pages)
{total: lengths | reduce(0, +), longest: lengths | reduce(max)}
```

```text
{longest: 412, total: 1757}
```

## Group by

`group(by = key)` partitions the rows by a key. Each group holds its `key` and
its `items`, and a `map` over the groups computes one row per group — SQL's
`GROUP BY` with its aggregates:

<!-- run: with shelf,views -->
```hql
shelf
| group(by = b => b.genre)
| map(g => {genre: g.key, books: g.items | size, pages: g.items | map(pages) | sum})
```

```text
{{books: 4, genre: "novel", pages: 1196}, {books: 1, genre: "poetry", pages: 145}, {books: 2, genre: "essay", pages: 416}}
```

`HAVING` is a `filter` after the `group`:

<!-- run: with shelf -->
```hql
shelf
| group(by = b => b.author)
| filter(g => size(g.items) > 1)
| map(g => g.key)
```

```text
{"Virginia Woolf"}
```

## Several answers in one pass

`tee` names a side branch of a pipeline. The branch reads the same rows, its
result is bound to a name, and the main pipeline goes on unchanged. A branch of
several steps continues on lines indented past the `|` before `tee`:

<!-- run: with shelf,views -->
```hql
reading = shelf
| tee unread =
    filter(b => b.status == "todo")
    | size
| tee read =
    filter(b => b.status == "done")
    | size
| filter(b => b.status == "reading")
| map(title)
{unread: unread, read: read, reading: reading}
```

```text
{read: 2, reading: ["Dune", "Mrs Dalloway"], unread: 3}
```

## From SQL to HQL

| SQL | HQL |
| --- | --- |
| `SELECT title` | `map(b => b.title)` |
| `WHERE status = 'todo'` | `filter(b => b.status == "todo")` |
| `ORDER BY year` | `sort(by = year)` |
| `LIMIT 3` | `take(3)` |
| `OFFSET 3` | `drop(3)` |
| `SELECT DISTINCT genre` | `map(b => b.genre) \| Set` |
| `COUNT(*)`, `SUM(pages)` | `size`, `map(pages) \| sum` |
| `GROUP BY genre` | `group(by = b => b.genre)` |
| `HAVING COUNT(*) > 1` | `filter(g => size(g.items) > 1)` |

## What is next

The shelf was written by hand. In [chapter 7](07-a-vault.md) the same books are
documents in a vault, and the same questions are asked of the documents
themselves.
