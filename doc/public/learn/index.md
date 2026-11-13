# Learn HQL

This is a book about HQL, read from the first page to the last. It starts with
a number and ends with a web of vaults, and every chapter builds on the ones
before it. Every program in it prints what the book shows. Chapter 18 and
chapters 25 to 27 describe APIs and object stores, the `ai` module and the
direction beyond one vault, which are design and not in the current
release, as do the marked sections of chapters 14, 15 and 19. Chapter 14's
companion on planning separates the local backend from future execution
strategies. The programs of chapters 16, 17, 19 and 20 that reach a
server are checked in the book and run in the
[hql-demo repository](https://github.com/ewiger/hql-demo).

HQL is a strongly typed language for querying, transforming, connecting and
computing over data, knowledge and AI workflows. A program is checked before it
runs, and values flow through pipelines of typed steps: over records and lists,
as `jq` works over JSON; over tables, as SQL does; over files, standard input
and streams that do not end; over a vault of Markdown documents, their links
and their meaning; and through the context, prompts and runs of a language
model.

The book is in four parts. **The language** is the steps, the values they
pass and the types that join them, from the first expression to the type
system and the boundary where data from outside becomes typed. **Data flows**
is the pipeline those steps form: what `|` does, when a program is checked,
where it runs, and the systems a pipeline connects. **Knowledge and AI** is
a vault of documents as a namespace, its graph, retrieval by meaning,
documents that compute themselves, agents whose runs are values, documents
that run them, and the web they point at. **The standard library** is the
modules a program imports for work that is neither a type nor I/O:
statistics, random values, digests and encodings, and cryptography.

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

The programs that reach a server — Kafka, PostgreSQL, NATS, a lakehouse —
run in the [hql-demo repository](https://github.com/ewiger/hql-demo). Each
of its cases is a vault with the Docker Compose file that starts what it
needs and a `run.sh` that compares the output with the expected one. With
Docker and `hql` on the `PATH`:

```sh
git clone https://github.com/ewiger/hql-demo && cd hql-demo
./run-all.sh
```

## Part I: The language

1. [Hello world](01-hello-world.md) — the first program, running it, and
   the check before a program runs.
2. [Data types](02-data-types/index.md) — `Int`, `Float`, `String` and
   `Bool` in the REPL, `Bytes`, `Data` and its JSON-shaped trees, `Option`
   for a value that may be missing, and how sets, `Data` and maps differ.
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
   `target` and `sink`, finite or live, formats, endpoints by id and the schema
   boundary, channels, logging, the network, files and folders, the
   environment and arguments, and `hql` as a command-line data tool.
10. [The type system](10-the-type-system/index.md) — records, unions and
    match, results, subtypes and narrowing, generics, receiver calls,
    operators and traits.
11. [Advanced data typing](11-advanced-data-typing/index.md) — the type
    boundary where loose data becomes typed, open and closed types,
    `TypeContract` and `DataContract`, validators, validation as a value,
    and what a contract states that a schema cannot.
12. [Errors](12-errors/index.md) — failures kept as data, diagnostics and the
    run's diagnostic channel, `strict` and `collect`.
13. [Working with `hql`](13-working-with-hql.md) — the commands, the reports,
    and what to do when something is wrong.

## Part II: Data flows

14. [ETL pipelines](14-etl-pipelines.md) — the wiring as a program: `|`
    and `.`, the three phases, logical steps and effects, `hql`
    in a shell pipe, the shop whose orders the rest of Part II follows, and
    partitioning, schedules and backends. Its companion,
    [Logical steps and physical plans](14-planning-and-backends.md), follows
    a checked program into local execution and the design direction for
    parallel workers, SQL translation and distributed execution.
15. [Endpoints](15-endpoints.md) — one model for every system: a file by
    path or by id, `source`, `tap`, `target` and `sink`, the kind that states
    the type, secrets in the environment, and transports.
16. [Tables and topics](16-tables-and-topics.md) — the shop's orders in a
    SQLite table, a PostgreSQL table and a Kafka topic, what a write
    promises, and a topic whose messages are not all orders.
17. [Subjects and the lakehouse](17-subjects-and-the-lakehouse.md) — core
    NATS notifications, a JetStream log and a durable consumer, and bronze,
    silver and gold Delta tables.
18. [APIs, webhooks and stores](18-apis-webhooks-and-stores.md) — a paginated
    REST API, a GraphQL catalogue, a webhook, an archive in object storage,
    and the systems the transport families reserve.
19. [OLTP](19-oltp/index.md) — the shop's transactional side: its whole day
    in one checked file, where its mistakes are found, a replay as the same
    file, and streams against tables.
20. [OLAP](20-olap/index.md) — the history those transactions leave: bronze,
    silver and gold, the grain of a row and its keys, deduplication, a table
    kept up to date by key, and incremental loads.

## Part III: Knowledge and AI

21. [A vault of documents](21-a-vault.md) — cards, their metadata, a reading
    list nobody maintains, and the vault as a namespace.
22. [Links and graphs](22-graphs.md) — following links, building graphs,
    drawing them.
23. [Finding cards by meaning](23-retrieval.md) — rankings, hits, and the
    evidence behind a score.
24. [Queries inside documents](24-documents.md) — cells, documents built
    from them, and cells that maintain a vault.
25. [Agents and runs](25-agents-and-runs.md) — agents as cards, context and
    prompts, tools, and runs kept as values.
26. [Agents inside documents](26-agents-inside-documents.md) — a document
    whose cells retrieve evidence, run an agent over the draft they sit in,
    and remember the answer.
27. [The hyper web](27-the-hyper-web.md) — namespaces beyond one tree,
    providers, retrieval and agents across them, and a reader who asks.

## Part IV: The standard library

28. [Statistics](28-statistics.md) — means, spread, quantiles, the mode and
    correlation, over a list and over a query's column.
29. [Random values](29-random.md) — sampling, shuffling, choice, numbers and
    UUIDs, and a seed that makes them reproducible.
30. [Digests and encodings](30-digests-and-encodings.md) — digests of text
    and bytes, the fingerprint of a `Data` tree, and hex, base64 and the
    other text encodings.
31. [Keys and cryptography](31-cryptography.md) — keys and their formats,
    secrets that print redacted, signatures, MACs and password hashes, and
    the encryption and key vault that are design.
