# Learn HQL

This is a book about HQL, read from the first page to the last. It starts with
a number and ends with a document that computes itself, and every chapter
builds on the ones before it. Every program in it runs with the current
release, and prints what the book shows.

HQL is a statically typed expression and query language. A program is checked
before it runs, and values flow through pipelines of typed steps: over records
and lists, as `jq` works over JSON; over tables, as SQL does; and over a vault
of Markdown documents, their links and their meaning.

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

1. [First steps](01-first-steps.md) — values, types, and the check before a
   program runs.
2. [Data: records and lists](02-data.md) — JSON-shaped trees: reading into
   them, reshaping them, writing them out.
3. [Collections](03-collections.md) — lists, sets and maps, and the steps they
   share.
4. [Functions](04-functions.md) — lambdas, declared and generic functions,
   aliases, and partial application.
5. [Asking questions](05-questions.md) — where, order by, group by and the
   aggregates.
6. [A vault of documents](06-a-vault.md) — cards, their metadata, and a reading
   list nobody maintains.
7. [Links and graphs](07-graphs.md) — following links, building graphs,
   drawing them.
8. [Finding cards by meaning](08-retrieval.md) — rankings, hits, and the
   evidence behind a score.
9. [Streams that do not end](09-streams.md) — generators, laziness, and
   batches.
10. [Writing results out](10-output.md) — presentation formats and who chooses
    them.
11. [Queries inside documents](11-documents.md) — cells, and documents built
    from them.
12. [Working with `hql`](12-working-with-hql.md) — the commands, the reports,
    and what to do when something is wrong.
