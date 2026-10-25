# Learn HQL

This is a book about HQL, read from the first page to the last. It starts with
a number and ends with an agent whose run is a value, and every chapter builds
on the ones before it. Every program in it prints what the book shows.
Chapter 15 covers the `ai` module, which ships in the next release; every
other chapter runs with the current one.

HQL is a statically typed language for querying, transforming, connecting and
computing over data, knowledge and AI workflows. A program is checked before it
runs, and values flow through pipelines of typed steps: over records and lists,
as `jq` works over JSON; over tables, as SQL does; over files, standard input
and streams that do not end; over a vault of Markdown documents, their links
and their meaning; and through the context, prompts and runs of a language
model.

## Following along

[Install `hql`](../install.md), clone the repository for the tutorial vault,
and run each program with `hql eval`:

```sh
git clone https://github.com/ewiger/hql && cd hql
hql --vault examples/tutorial eval 'cards | size'
```

The tutorial vault, [`examples/tutorial/`](https://github.com/ewiger/hql/tree/main/examples/tutorial),
is a small reading club: seven books, five authors, the links between them,
and one claim about which book inspired which.

## Chapters

1. [Hello world](01-hello-world.md) — the first program, running it, and
   the check before a program runs.
2. [Data types](02-data-types/index.md) — `Int`, `Float`, `String` and
   `Bool` in the REPL, `Data` and its JSON-shaped trees, `Option` for a value
   that may be missing, and how sets, `Data` and maps differ.
3. [Collections](03-collections.md) — lists, sets and maps, and the steps they
   share.
4. [Functions](04-functions/index.md) — lambdas, declared functions, the
   two body forms `= expr` and `{ … }`, generic functions, aliases, and
   partial application.
5. [Modules](05-modules/index.md) — modules of your own, imports, receiver
   modules, public and private declarations, and how a name resolves.
6. [SQL-like Queries](06-sql-like-queries.md) — where, order by, group by and the
   aggregates.
7. [A vault of documents](07-a-vault.md) — cards, their metadata, and a reading
   list nobody maintains.
8. [Links and graphs](08-graphs.md) — following links, building graphs,
   drawing them.
9. [Finding cards by meaning](09-retrieval.md) — rankings, hits, and the
   evidence behind a score.
10. [Streams that do not end](10-streams.md) — generators, laziness, and
    batches.
11. [Writing results out](11-output.md) — presentation formats and who chooses
    them.
12. [Queries inside documents](12-documents.md) — cells, and documents built
    from them.
13. [Working with `hql`](13-working-with-hql.md) — the commands, the reports,
    and what to do when something is wrong.
14. [I/O](14-io/index.md) — sources and targets: endpoints, `source`, `tap`,
    `target` and `sink`, finite or live, formats, sources by id and the schema
    boundary, channels, logging, the network, and `hql` as a command-line data
    tool.
15. [Agents and runs](15-agents-and-runs.md) — agents as cards, context and
    prompts, tools, and runs kept as values.
16. [The type system](16-the-type-system/index.md) — records, unions and
    match, results, subtypes and narrowing, generics, receiver calls,
    operators and traits.
17. [Advanced data typing](17-advanced-data-typing/index.md) — the type
    boundary where loose data becomes typed, open and closed types,
    `TypeContract` and `DataContract`, validators, validation as a value,
    and what a contract states that a schema cannot.
