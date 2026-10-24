# HQL { .hql-hero }

HQL, the Hyper Query Language, is a statically typed expression and query
language. It combines functional expressions with pipelines over files,
streams, structured data and HyperMarkDown documents, and checks a program's
types before it evaluates it.

```hql
type Book { title : String, year : Int, rating : Int }

books = [
    Book(title = "Dune", year = 1965, rating = 5),
    Book(title = "Emma", year = 1815, rating = 3),
    Book(title = "Beloved", year = 1987, rating = 4),
]

books
| filter(b => b.rating >= 4)
| sort(b => b.title)
| map(b => b.title)
```

```text
["Beloved", "Dune"]
```

`source` reads the documents a file holds, and `target` writes a value to one:

```hql
source(file("sightings.jsonl"), Json)
| filter(s => s.seen == "morning")
| target(file("morning.json"))
```

<div class="grid cards" markdown>

- **[Install](install.md)** — a release binary or `cargo install hql`.
- **[Learn HQL](learn/index.md)** — the book, one runnable program at a
  time.
- **[Reference](reference.md)** — the grammar, the command line, and
  reporting.

</div>
