# 8. Writing results out

A program's value is computed first, and written second. How it is written is
a **presentation format** — plain text, HQL, Markdown, JSON, YAML, CSV, or a diagram
— and the format is a type, so a program that formats a value knows what kind
of text it holds.

The programs of this chapter read the shelf of
[chapter 6](06-sql-like-queries.md), with its typed views, and the book shows
these lines once:

<!-- define: shelf -->
<!-- run: skip -->
```hql
shelf = [
    {title: "Frankenstein", author: "Mary Shelley", year: 1818, genre: "novel", pages: 280, status: "done"},
    {title: "Dune", author: "Frank Herbert", year: 1965, genre: "novel", pages: 412, status: "reading"},
    {title: "The Hobbit", author: "J. R. R. Tolkien", year: 1937, genre: "novel", pages: 310, status: "todo"},
    {title: "Leaves of Grass", author: "Walt Whitman", year: 1855, genre: "poetry", pages: 145, status: "todo"},
    {title: "A Room of One's Own", author: "Virginia Woolf", year: 1929, genre: "essay", pages: 112, status: "done"},
    {title: "The Common Reader", author: "Virginia Woolf", year: 1925, genre: "essay", pages: 304, status: "todo"},
    {title: "Mrs Dalloway", author: "Virginia Woolf", year: 1925, genre: "novel", pages: 194, status: "reading"},
]
fn title(b: Data) : String = match b.title { some(String t) => t, _ => "" }
fn pages(b: Data) : Int = match b.pages { some(Int n) => n, _ => 0 }
```

## Formats are a category of their own

Each format is a `Format`: `Text`, `Lines`, `Hql`, `Markdown`, `Hmd`, `Json`,
`JsonLines`, `Yaml`, `Csv`, `D2` and `Mermaid`. A format describes how values
are written, not the text itself: a step named for a format encodes any value
in it, and what comes out is a `String`:

<!-- run: with shelf -->
```hql
shelf | filter(b => b.status == "todo") | map(title) | markdown
```

```text
- The Hobbit
- Leaves of Grass
- The Common Reader
```

<!-- run: check with shelf -->
```hql
shelf | filter(b => b.status == "todo") | map(title) | markdown
```

```text
String
```

A formatted value is a string that remembers its format, so a host prints it
as it is. A later step may read it, and comparing it with a string compares
the text:

```hql
([1, 2] | json) == "[\n  1,\n  2\n]"
```

```text
true
```

## The longhand, `format`

`markdown` is the shorthand for `format(Markdown)`. The longhand takes two
options. `limit` keeps the first `n` elements and says how many were cut:

<!-- run: with shelf -->
```hql
shelf
| map(title)
| format(Text, limit = 3)
```

```text
["Frankenstein", "Dune", "The Hobbit"]
… 4 more not shown
```

A list of records in Markdown is a bullet list. A table is a value of its own
type, `Table`: `Table(xs, columns = [...])` takes one row per record and the
columns in the order written, and every format writes it as a table, a pipe
table in Markdown:

<!-- run: with shelf -->
```hql
shelf
| map(b => {title: title(b), pages: pages(b)})
| Table(columns = ["title", "pages"])
| markdown
```

```text
| title               | pages |
| ------------------- | ----- |
| Frankenstein        | 280   |
| Dune                | 412   |
| The Hobbit          | 310   |
| Leaves of Grass     | 145   |
| A Room of One's Own | 112   |
| The Common Reader   | 304   |
| Mrs Dalloway        | 194   |
```

The same table in `Csv` is a header row and one row per record, in `Json` an
array of objects with their keys in column order, and in `Text` space-aligned
columns. `renderer` names one of a format's encoders for a shape over the
project's choice, `format(Markdown, renderer = "mermaid")` for a graph; a table
renderer, such as `pipe`, writes only a `Table`.

A format that cannot write a value is refused before the program runs:

```hql
1 | d2
```

```text
type error: no renderer writes Int as D2
 --> <expression>:1:5
  |
1 | 1 | d2
  |     ^^
```

## Choosing the format from outside

A program that does not format its result is written as HQL — the literal you
would type to get the value back, a string in quotes. `--to text` writes a
string bare instead, and `--to` names any other format without changing the
program:

<!-- run: with shelf --to markdown -->
```hql
shelf | filter(b => b.status == "todo") | map(title)
```

```text
- The Hobbit
- Leaves of Grass
- The Common Reader
```

`--to none` evaluates the program and writes nothing. `--envelope json` wraps
the answer for another program to read: the type, the value, and every report:

<!-- run: with shelf --envelope json -->
```hql
shelf | filter(b => b.status == "todo") | size
```

```text
{
  "origin": "<expression>",
  "reports": [],
  "type": "Int",
  "value": 3
}
```

The order is fixed: a `format` step in the program wins, then `--to`, then the
host's own — Markdown for a document, HQL for a terminal.

## A project chooses its renderers

Which renderer writes a type in a format is a project's choice, in the
`hql.toml` of the vault a program runs in ([chapter 15](15-a-vault.md)), over
the built-in defaults. A graph in Markdown is a D2 diagram unless the file
says otherwise:

```toml
[format.Graph]
markdown = "mermaid"
```

A renderer or format the binary does not have fails loading the vault, so the
mistake is reported before any query runs.

## What is next

Every value so far was written to the terminal. [Chapter 9](09-io/index.md)
reads files, standard input and the network, and writes files and standard
output, without leaving the language.
