# 6. A vault of documents

The shelf of [chapter 5](05-questions.md) was a list written by hand. Here the
same books are documents in a folder, and HQL reads the folder.

## A vault is a folder

A **vault** is a directory of Markdown and HyperMarkDown documents. The tutorial
vault looks like this:

```text
examples/tutorial/
├── authors/
│   ├── frank-herbert.hmd
│   ├── jrr-tolkien.hmd
│   ├── mary-shelley.hmd
│   ├── virginia-woolf.hmd
│   └── walt-whitman.hmd
├── books/
│   ├── a-room-of-ones-own.hmd
│   ├── dune.hmd
│   ├── frankenstein.hmd
│   ├── leaves-of-grass.hmd
│   ├── mrs-dalloway.hmd
│   ├── the-common-reader.hmd
│   └── the-hobbit.hmd
├── club.hmd
├── frankenstein-inspires-dune.hmd
└── hql.toml
```

Each document is a **card**. A card has a header of metadata and a body
someone wrote:

```markdown
---
title: Dune
metadata:
  category: book
  author: Frank Herbert
  year: 1965
  genre: novel
  pages: 412
  status: reading
---
# Dune

A novel by [[frank-herbert]]. A desert planet, a spice, and a family that inherits both.
```

`--vault <directory>` names the vault a program runs against:

```sh
hql --vault examples/tutorial eval 'cards | size'
```

```hql
cards | size
```

```text
14
```

## `cards`

`cards` is every card of the vault, as a sequence. Its order is the order a
sorted walk of the folder reaches each document — the entries of each
directory by name, depth first — so a program answers the same on every
machine:

```hql
cards | take(4) | map(c => c.name)
```

```text
["frank-herbert", "jrr-tolkien", "mary-shelley", "virginia-woolf"]
```

A card has the fields of a document: its `name` (the file stem), `title`,
`path`, `format`, `body`, its `kind`, and `metadata`, the header's tree.

## A link is an optional card

`[[name]]` refers to a card by name, the way a link in a document does. A link
may point at a document nobody has written yet, so its type is `Option<Card>`,
and reading a field through it reads through the option:

```hql
[[dune]].title
```

```text
"Dune"
```

```hql
[[nowhere]].title
```

```text
none
```

## Metadata is Data

A card's metadata is the `Data` of [chapter 2](02-data.md), so every question
from chapters 2 and 5 applies. The books are the cards whose category says so:

```hql
cards
| filter(c => c.metadata.category == "book")
| map(c => c.title)
```

```text
["A Room of One's Own", "Dune", "Frankenstein", "Leaves of Grass", "Mrs Dalloway", "The Common Reader", "The Hobbit"]
```

## The reading list

The club's reading list is the books still to read, by title:

```hql
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```

```text
["Leaves of Grass", "The Common Reader", "The Hobbit"]
```

Read it as three steps, each consuming the one before. `filter` decides each
card on its own; `sort` needs every card before it can place the first; `map`
turns each card into its title. A step that decides each value alone may run
on any part of the input in any order, and a step that needs the whole input
is where the parts meet. That is the shape of **MapReduce**, and HQL keeps it
visible in the pipeline: nothing is stored in between, and nothing is a loop.

No one maintains this list. Mark a book `status: done` in its header and the
next run of the query no longer lists it.

## Grouping cards

`group` partitions the books by a header value, as it partitioned the shelf:

```hql
cards
| filter(c => c.metadata.category == "book")
| group(by = c => c.metadata.status)
| map(g => {status: g.key, books: g.items | map(c => c.title)})
```

```text
{{books: ["A Room of One's Own", "Frankenstein"], status: "done"}, {books: ["Dune", "Mrs Dalloway"], status: "reading"}, {books: ["Leaves of Grass", "The Common Reader", "The Hobbit"], status: "todo"}}
```

A typed view of a card works as the views of chapter 5 did, and the pages per
author are a map over each group and a sum — map, then reduce:

```hql
fn pages(c: Card) : Int = match c.metadata.pages { some(Int n) => n, _ => 0 }

cards
| filter(c => c.metadata.category == "book")
| group(by = c => c.metadata.author)
| map(g => {author: g.key, pages: g.items | map(pages) | sum})
```

```text
{{author: "Virginia Woolf", pages: 610}, {author: "Frank Herbert", pages: 412}, {author: "Mary Shelley", pages: 280}, {author: "Walt Whitman", pages: 145}, {author: "J. R. R. Tolkien", pages: 310}}
```

## Kinds of card

A card whose header declares a relation between two other cards is a
`RelationCard`; every other card is a `ConceptCard`. `of_type` keeps the cards
of one kind:

```hql
cards | of_type(RelationCard) | map(c => c.title)
```

```text
["Frankenstein inspires Dune"]
```

[Chapter 7](07-graphs.md) is about what that relation means: cards and the
links between them form a graph.

## The vault itself

`cards` is a binding the standard library makes, `cards = vault.cards`, over
`vault`, the value the host seeds when it is given a vault:

```hql
[vault.name, vault.cards | size | text]
```

```text
["tutorial", 14]
```

Without `--vault` there is no vault, and naming one says so:

<!-- run: novault -->
```hql
cards | size
```

```text
name error: `cards` needs a vault: pass --vault <directory>
 --> <expression>:1:1
  |
1 | cards | size
  | ^^^^^
```
