# Learn HQL

This is a book about HQL, read from the first page to the last. It starts with
a number and ends with a web of vaults, and every chapter builds on the ones
before it. Every program in it prints what the book shows. Chapters 18 to 20
describe the `ai` module and the direction beyond one vault, which are design
and not in the current release; every other chapter runs with it.

HQL is a statically typed language for querying, transforming, connecting and
computing over data, knowledge and AI workflows. A program is checked before it
runs, and values flow through pipelines of typed steps: over records and lists,
as `jq` works over JSON; over tables, as SQL does; over files, standard input
and streams that do not end; over a vault of Markdown documents, their links
and their meaning; and through the context, prompts and runs of a language
model.

The book is in three parts. **The language** is the steps, the values they
pass and the types that join them, from the first expression to the type
system and the boundary where data from outside becomes typed. **Data flows**
is the pipeline those steps form: what `|` does, when a program is checked,
where it runs, and the systems a pipeline connects. **Knowledge and AI** is
a vault of documents as a namespace, its graph, retrieval by meaning,
documents that compute themselves, agents whose runs are values, documents
that run them, and the web they point at.

## Following along

[Install `hql`](../install.md), clone the repository for the tutorial vault,
and run each program with `hql eval`:

```sh
git clone https://github.com/ewiger/hql && cd hql
hql --vault examples/tutorial eval 'cards | size'
```

The tutorial vault, [`examples/tutorial/`](https://github.com/ewiger/hql/tree/main/examples/tutorial),
is a small reading club: seven books, five authors, the links between them,
one claim about which book inspired which, and a librarian.

## Part I: The language

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
7. [Streams that do not end](07-streams.md) — generators, laziness, and
   batches.
8. [Writing results out](08-output.md) — presentation formats and who chooses
   them.
9. [I/O](09-io/index.md) — sources and targets: endpoints, `source`, `tap`,
   `target` and `sink`, finite or live, formats, sources by id and the schema
   boundary, channels, logging, the network, and `hql` as a command-line data
   tool.
10. [The type system](10-the-type-system/index.md) — records, unions and
    match, results, subtypes and narrowing, generics, receiver calls,
    operators and traits.
11. [Advanced data typing](11-advanced-data-typing/index.md) — the type
    boundary where loose data becomes typed, open and closed types,
    `TypeContract` and `DataContract`, validators, validation as a value,
    and what a contract states that a schema cannot.
12. [Working with `hql`](12-working-with-hql.md) — the commands, the reports,
    and what to do when something is wrong.

## Part II: Data flows

13. [Pipelines](13-pipelines.md) — `|` against `.`, the three phases, the
    step as the unit of execution, `hql` in a shell pipe, and the systems a
    pipeline will connect: queues, databases, services and jobs.

## Part III: Knowledge and AI

14. [A vault of documents](14-a-vault.md) — cards, their metadata, a reading
    list nobody maintains, and the vault as a namespace.
15. [Links and graphs](15-graphs.md) — following links, building graphs,
    drawing them.
16. [Finding cards by meaning](16-retrieval.md) — rankings, hits, and the
    evidence behind a score.
17. [Queries inside documents](17-documents.md) — cells, documents built
    from them, and cells that maintain a vault.
18. [Agents and runs](18-agents-and-runs.md) — agents as cards, context and
    prompts, tools, and runs kept as values.
19. [Agents inside documents](19-agents-inside-documents.md) — a document
    whose cells retrieve evidence, run an agent over the draft they sit in,
    and remember the answer.
20. [The hyper web](20-the-hyper-web.md) — namespaces beyond one tree,
    providers, retrieval and agents across them, and a reader who asks.
