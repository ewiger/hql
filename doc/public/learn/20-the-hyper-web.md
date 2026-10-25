# 20. The hyper web

!!! warning "Design only"

    This chapter describes functionality that is not in the current release.
    It is written as if it existed: nothing in it runs against the current
    binary, and names, types and outputs may change before any of it ships.
    It is the direction the rest of the book points in, stated so that the
    direction can be read, questioned and changed.

A vault is one namespace: a tree of cards, closed by construction, in which
every `[[name]]` resolves or is reported. The chapters of Part III queried,
drew, ranked and ran agents over one such tree. This chapter asks what each
of those steps is over many trees, authored and served independently, and
what a reader is when the trees are read by programs and agents as much as by
people.

## Namespaces beyond one tree

A reference may name a namespace: `[[club:dune]]` is the card `dune` in the
namespace bound as `club`. The binding lives in the vault's configuration,
never in a card, so a card cannot reach a namespace by mentioning it, and
removing the binding breaks every link that used it, visibly. A prefix is a
permission.

```toml
[namespaces.club]
root = "../reading-club"

[namespaces.lit]
url = "https://lit.example.org/"
```

HQL already has the local half of this. `source("club")` in
[chapter 14](14-a-vault.md) is a vault bound by configuration, with its own
`cards` and `graph`, and a relation across the two is a join. A namespace
bound to a URL is the same source whose provider is a server; the program is
unchanged:

<!-- run: skip -->
```hql
lit = source("lit")
lit.cards | filter(c => c.metadata.category == "book") | size
```

The rule the design rests on is no silent dereference: rendering, checking and
linting make no network call, and a program that reads a remote namespace does
so in a step the host sees and may refuse, as it sees and may refuse
`source(http(…))` today.

## Providers

A **provider** is anything that returns a card for a path: the vault's own
folder, another folder, a web server that speaks plain HTTP and publishes a
route description so a client does not guess its layout, a database holding
cards as rows. The card's name is the same whichever provider answers, so a
program over `cards` is a program over a web of vaults, and where a card is
stored stays out of every reference to it.

What arrives from another provider is untrusted. A remote card's header is
`Data`, as every tree from outside a program is, and it becomes a typed value
at a boundary of [chapter 11](11-advanced-data-typing/index.md), where the
program states what it requires and the rest is refused or kept as `Data`.
A card of a namespace carries the namespace in its `provenance`, and so does
every value derived from it: a hit, a context item, a run.

## Retrieval over the web

`semantic` over a namespace is the provider's index answering a question it
holds, with its model, metric and approximation reported in the ranking's
`retrieval`, as chapter 16 reports them for a local index. Two namespaces are
two rankings, and a program that combines them does so explicitly, by score
within a retrieval or by a join across them, because a score is evidence
relative to one index and means nothing against another's.

<!-- run: skip -->
```hql
import semantic

here  = cards | semantic("a creature and its maker") | take(3)
there = source("lit").cards | semantic("a creature and its maker") | take(3)
[here, there] | emit | Context("evidence")
```

Each item of that context says which namespace it came from, which index
ranked it and how. Knowledge semantics becomes a way to query the web: a
question in words, answered with cards and the evidence for each, rather than
with pages to read.

## Agents reading agents

An agent is a card, so a namespace can serve one, and a document in one vault
can run the librarian of another. `delegate` across a binding is a run whose
child ran elsewhere, kept in `children` with its provenance, and whose `trace`
records the delegation as an event like any other. The rules of chapter 18 do
not change at the boundary: a child run is bounded in depth, every request is
recorded under its hash, and a tool stays pure wherever it is evaluated.

<!-- run: skip -->
```hql
import ai

review = cards | lexical("influence") | Context("claims") | run(@[[librarian]] | Agent)
second = review | delegate(@[[lit:reviewer]] | Agent, "Check the claims against your shelf.")
second.trace | filter(e => e.kind == "delegate") | map(e => e.data.namespace)
```

An agent that reads another agent's run reads a value: the prompt it was
given, the tools it called, the answer it gave, and where each piece came
from. A conversation between agents is a tree of such values, and a program
queries it with the steps of chapter 6.

## A reader who asks

The same card is served to a person in a browser, to a tool that lints it, and
to an agent building a context, from one source and in one format. A site
that is a vault answers a question the way a document of chapter 19 does:
with a run whose evidence is the site's own cards, whose trace says which
cards were read, and whose answer carries their names. The reader asks instead
of browsing, and what comes back is not a page but a value with its
provenance attached, which the reader, or the reader's agent, may query
further.

None of this needs a new language. It needs bindings in configuration,
providers that answer by path, and the discipline the earlier chapters already
hold: every step typed, every effect explicit, every value carrying where it
came from.

## Where to go from here

You have met the whole language, in three parts. The language: values and
data, collections and functions, modules, questions in the style of SQL,
streams, formats, data read and written through endpoints, the types a
program declares and the boundary where data becomes typed, and the tools
around it. Data flows: the pipeline, its phases, its unit of execution, and
the systems it will connect. Knowledge and AI: a vault that is a namespace,
its graph, retrieval by meaning, documents that compute themselves, agents
whose runs are values, documents that run them, and the web they point at.
The [reference](../reference/index.md) states each shipped part precisely,
and the [examples](../examples.md) are where the next part is designed.
