# Learn HQL

This is a book about HQL, read from the first page to the last. It starts with
a number and ends with an agent whose run is a value, and every chapter builds
on the ones before it. Every program in it prints what the book shows.
Chapters 14 to 16 cover the `io`, `io.logging` and `ai` modules, which ship
in the next release; every other chapter runs with the current one.

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
14. [Reading and writing data](14-reading-and-writing.md) — endpoints,
    `source`, `tap`, `target` and `sink`, and `hql` as a command-line data tool.
15. [Channels and logs](15-channels-and-logs.md) — values passed through
    channels, and structured logging.
16. [Agents and runs](16-agents-and-runs.md) — agents as cards, context and
    prompts, tools, and runs kept as values.
17. [Types and traits](17-types-and-traits/index.md) — records, unions,
    results, receiver calls, traits and operators.
18. [Errors](18-errors/index.md) — failures kept as data, diagnostics and the
    run's diagnostic channel, `strict` and `collect`.
