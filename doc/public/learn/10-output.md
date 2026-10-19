# 10. Writing results out

A program's value is computed first, and written second. How it is written is
a **presentation format** — plain text, HQL, Markdown, JSON, YAML, CSV, or a diagram
— and the format is a type, so a program that formats a value knows what kind
of text it holds.

## Formats are types

Each format is a subtype of `String`: `Text`, `Hql`, `Markdown`, `Hmd`, `Json`,
`Yaml`, `Csv`, `D2` and `Mermaid`. A step named for a format writes any value in it:

```hql
cards | filter(c => c.metadata.status == "todo") | map(c => c.title) | markdown
```

```text
- Leaves of Grass
- The Common Reader
- The Hobbit
```

<!-- run: check -->
```hql
cards | filter(c => c.metadata.status == "todo") | map(c => c.title) | markdown
```

```text
Markdown
```

A formatted value is still a value. A later step may read it, and comparing it
with a string compares the text:

```hql
([1, 2] | json) == "[\n  1,\n  2\n]"
```

```text
true
```

A card written as Markdown is its body; a list of cards, a list of links:

```hql
cards | filter(c => c.metadata.category == "author") | markdown
```

```text
- [[frank-herbert]]
- [[jrr-tolkien]]
- [[mary-shelley]]
- [[virginia-woolf]]
- [[walt-whitman]]
```

## The longhand, `format`

`markdown` is the shorthand for `format(Markdown)`. The longhand takes two
options. `limit` keeps the first `n` elements and says how many were cut:

```hql
cards
| filter(c => c.metadata.category == "book")
| map(c => c.title)
| format(Text, limit = 3)
```

```text
["A Room of One's Own", "Dune", "Frankenstein"]
… 4 more not shown
```

`renderer` chooses how a value is written in the format. A list of records in
Markdown is a list by default, and a table with `renderer = "pipe"`:

```hql
cards
| filter(c => c.metadata.category == "book")
| map(c => {title: c.title, pages: c.metadata.pages})
| format(Markdown, renderer = "pipe")
```

```text
| pages | title               |
| ----- | ------------------- |
| 112   | A Room of One's Own |
| 412   | Dune                |
| 280   | Frankenstein        |
| 145   | Leaves of Grass     |
| 194   | Mrs Dalloway        |
| 304   | The Common Reader   |
| 310   | The Hobbit          |
```

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

<!-- run: --vault examples/tutorial --to markdown -->
```hql
cards | filter(c => c.metadata.status == "todo") | map(c => c.title)
```

```text
- Leaves of Grass
- The Common Reader
- The Hobbit
```

`--to none` evaluates the program and writes nothing. `--envelope json` wraps
the answer for another program to read: the type, the value, and every report:

<!-- run: --vault examples/tutorial --envelope json -->
```hql
cards | filter(c => c.metadata.status == "todo") | size
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

## A vault chooses its renderers

Which renderer writes a type in a format is the vault's choice, in its
`hql.toml`, over the built-in defaults. A graph in Markdown is a D2 diagram
unless the vault says otherwise:

```toml
[format.Graph]
markdown = "mermaid"
```

A renderer or format the binary does not have fails loading the vault, so the
mistake is reported before any query runs.

## What is next

A document is where most results end up. [Chapter 11](11-documents.md) puts
queries inside documents and builds the documents they describe.
