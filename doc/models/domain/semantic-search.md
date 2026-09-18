# The semantic search domain

Status: **partly implemented.** Steps one to five of the path at the end of this
document are built and tested: `Seq`, `take` with a stated order, `Hit`,
`Ranking` and `Retrieval`, an offline exact index, and `expand`. What remains is
the execution contract for an approximate store, `recall` over knowledge, and
the whole write path. Where this document says something is missing, check the
path before assuming it still is; the vocabulary below is now real syntax.

The other two domain documents are the [graph model](graphs.md), which is
structural, and the [knowledge model](knowledge.md), which is semantic. This one
is **selective**: it orders a corpus against a query and hands back a prefix.

It is a domain in the sense that it has a vocabulary of its own, and it is *not*
a peer of the other two in the sense the [type system](../../wiki/hql/type-system.hmd)
uses, because it introduces no node type and no edge type. It mints exactly one
new shape — a ranked hit — and everything that shape claims is a knowledge
claim. The rule that a supertype set may not join types from different domains
is therefore not at risk here: nothing in this document is anyone's parent.

This is the **read path of AI memory**. The write path is sketched at the end and
is the larger unfinished half.

## The pipeline this model exists to explain

```hql
cards
| semantic("bearer token authorization")
| take(5)
| graph(depth = 1)
```

Four stages, and the current documentation supports none of them. Read
left to right, the missing parts are: a ranked collection type, a prefix
operation over it, an index that is not the vault, a traversal vocabulary, and a
decision about what `graph` means when it is given both a collection and a depth.

The form this model recommends separates the last stage into two:

```hql
cards
| semantic("bearer token authorization")
| take(5)
| expand(depth = 1)
| graph
```

This is the form that was implemented, and it runs today. `expand` is traversal
and `graph` stays the projection it is in the knowledge model. Folding both into one name would settle the open naming question in
[graphs.md](graphs.md) by accident, in the direction that loses the distinction.

## Vocabulary

```text
Query        what is being searched for
Embedding    a vector representation, produced by a named model
Index        a searchable store over a corpus, with a revision and a horizon
Metric       how two embeddings are compared
Score        a metric result — always relative to one query
Hit[T]       a value, its score, and the retrieval that produced both
Ranking[T]   an ordered collection of hits with a stated tie rule
Retrieval    the retained rule: query, index, model, metric, approximation
```

```hql
type Hit[T] {
    value      : T
    score      : Score
    provenance : Retrieval
}

type Retrieval {
    query       : Query
    index       : IndexRef
    model       : ModelRef
    metric      : Metric
    approximate : Bool
}

type Ranking[T] <: Seq[Hit[T]]
```

`Ranking` narrows `Seq` the way `Relation` narrows `Edge`: it adds what makes the
order meaningful — the query the order is relative to, and the rule that broke
ties. A bare `Seq[Hit[T]]` is an ordered pile of scored things with no statement
about where the order came from.

`Query` is a type, not a `String`. HQL has no implicit conversions, so
`semantic("...")` either takes a `String` and constructs the query, or the
literal is elaborated at the call — a grammar decision, not a free reading.

## A score is evidence, never relevance

This is the whole of the interaction with the knowledge domain, and everything
else follows from it.

The [knowledge model](knowledge.md) rests on one rule: interpretation is a step,
and the rule used must be retained with the result. A similarity score is an
interpretation. It was produced by a named model over a particular chunking under
a particular metric against an index at a particular revision. Drop any of that
and what remains is an assertion that a card is relevant, with nothing behind it.

So the knowledge-domain reading of a hit is:

```text
Proposition   Relevant(card, query)
Evidence      the Hit — score, model, index revision, metric
Provenance    which index answered, and when
```

**Provenance confers nothing.** A card scoring 0.91 is not more true than one
scoring 0.42; it is more similar to a query under one model, which is a different
statement and is not about the card's content being correct. The knowledge model
already forbids a projection turning an assertion into a truth, and a ranked list
presented without its retrieval is exactly that conversion.

This is also what makes `Retrieval` non-optional on `Hit`. A hit that cannot say
how it was produced is not evidence.

## What an index contributes, and what it cannot

[Card](../../wiki/hql/types/card-type.hmd) and [fields](../../wiki/hql/fields.hmd)
already give the index a home: `card.metadata` is assembled when a document
becomes a card, from the authored block, system metadata, extension
contributions, `.hmd/**` vault state and other derived information — and
"semantic-search state" is named among the inputs.

That home is correct for exactly one half of it:

| Belongs in `card.metadata` | Cannot | Why |
| --- | --- | --- |
| Embeddings, chunk identities | | a property of the card |
| Index membership, indexed revision | | a property of the card |
| Staleness against the card's content | | a property of the card |
| | A score | exists only relative to a query |
| | A rank | exists only relative to a query and a corpus |

A card has an embedding whether or not anyone ever searches. It has no score
until someone asks something. Storing a score in an assembled per-card layer
would mean either one privileged query or a stale number that no longer means
what its name says.

[Card](../../wiki/hql/types/card-type.hmd) currently lists "an indexer supplying
embeddings or a semantic-search score" as a single kind of header contribution.
The first is a contribution and the second is a category error; the sentence
needs splitting, recorded as `INC-23`.

Where the index state lands also has to answer Card metadata precedence, which
[consolidation](../../status/consolidation.md) tracks as `CON-07`: an authored
`search.exclude` entry and an extension-contributed index membership are two
inputs to one path, and which wins is not decided.

## Coverage: cards are not concepts

`cards | semantic(...)` ranks **representations**. AI memory wants to recall
**knowledge**. The knowledge model is explicit that these differ: `Card` is the
base representation type, `ConceptCard <: {Card, Concept}` occupies a node
position, and `Graph[Concept, Relation]` is parameterized by `Concept` rather
than `ConceptCard` precisely because a concept can exist with no card at all —
imported, derived, or read from a store.

Those cardless concepts have no text. Text search cannot reach them, and a memory
that silently cannot recall part of itself is worse than one that says so. Two
operations, not one:

```hql
semantic(q) : Set[Card]  -> Ranking[Card]      // over representations
recall(q)   : Knowledge  -> Ranking[Concept]   // over knowledge
```

`recall` is the AI-memory operation. It needs a stated answer for how a concept
with no document is embeddable — from its relations, its propositions, the cards
that mention it — and that answer is itself a derivation whose rule must be
retained. It is not a detail of the index; it is a claim about what the concept
is *like*.

## Seeding a graph from hits

`take(5)` yields five cards. Turning them into graph seeds is not uniform,
because the card family is not uniform:

- A **`ConceptCard`** is a `Concept` and seeds a node directly.
- A **`RelationCard`** is not a node. It contributes an edge, and the knowledge
  model is explicit that making it both would put every explained relationship
  in the graph twice.

So a retrieval that hits a relation card has three candidate readings: drop it,
seed both of its relation's endpoints, or contribute the edge and let expansion
supply the nodes. The one reading that must not be the default is the one a naive
implementation reaches for — treating every hit as a node — because it
reintroduces the duplication `RelationCard <: Card` exists to prevent.

A `recall` over concepts does not have this problem, which is a second argument
for it being the memory operation.

## Expansion, and why the result must say why each node is there

Traversal does not exist. [graphs.md](graphs.md) states it outright: no
traversal, subgraph or path vocabulary has been designed. `expand(depth = 1)`
needs at minimum:

- **Direction** — `uplinks`, `downlinks`, or both. `depth = 1` says nothing, and
  the undirected reading is a choice, not the absence of one.
- **Edge set** — authored links only, relations only, or everything an extension
  supplies. A document graph and a knowledge graph expand differently.
- **Horizon** — the open question graphs.md already records. `cards` stops at the
  vault; a seed's neighbour may not.
- **Named-argument grammar** — `depth = 1` is syntax nothing has decided.

And then the part that is a knowledge problem rather than a graph problem:
expanding five hits at depth one produces a subgraph in which most nodes were
never scored. If the result is typed `KnowledgeGraph` it must retain what it
knows, so each node carries why it is present:

```text
retrieved   with its Hit
expanded    with the seed and the path that reached it
```

Without that, the subgraph asserts "these fourteen cards are about bearer token
authorization", which is false for nine of them — an assertion manufactured by a
projection, which is the one thing `KnowledgeGraph` is typed to prevent.

## Execution: approximation is semantics, not optimization

[pipes](../../wiki/hql/types/pipes.hmd) reserves source-side query execution as
executor freedom, and in the same breath requires that an optimization "preserve
observable values". Approximate nearest-neighbour search does not preserve
observable values. Under the current text, pushing `semantic | take` into a store
is therefore illegal, and the only legal reading is the one no index implements:
enumerate the vault, score every card, sort, keep five.

The resolution is not to weaken the optimizer rule. It is that **`semantic`
declares itself approximate in its own contract**, so the difference is part of
what the stage means rather than a liberty the executor took. `approximate` is a
field on `Retrieval` for the same reason: a consumer can see which answer it got.

Two further consequences, both landing on decisions
[design status](../../wiki/hql/design-status.hmd) already lists as needed under
extension transport and optimizer-visible execution properties:

- **Top-k fusion.** `semantic | take(n)` is the operation a store implements. The
  fusion is only available if `take` is visible to the stage that precedes it,
  which is a contract question, not a rewrite rule.
- **Purity.** The same query against a changed index gives different answers.
  Pinning an index revision makes `semantic` reproducible; without pinning it is
  an effect, and the eager/lazy boundary cannot be drawn around it.

**Ties.** `Score` is a `Float`, and ties are ordinary at realistic precision.
`take(5)` over an unbroken tie is non-deterministic, which
[collections](../../wiki/hql/collections.hmd) already forbids: a reproducible
prefix requires a defined order including equal sort keys. A `Ranking` must state
its tiebreak — a stable card key is the obvious candidate. Note this does not
touch `CON-05`: a `Ranking` is a `Seq` because a query imposed an order, which is
not an argument that `cards` was ever ordered.

## AI memory: the write path

Recall is the half this document can nearly specify. Remembering is not, and the
knowledge model names the gap exactly: constructing knowledge is pure,
contributing it to a store is not, and the boundary has not been drawn.

What memory needs beyond that boundary:

**Reconciliation.** A new memory about an existing concept must find that
concept, and finding it is a retrieval. That closes a loop worth naming: search
is used to *propose* concept identity, while identity is a knowledge claim. A
score may be evidence that two concepts are the same; it may never be the
decision. Merging them is a derivation whose rule must be retained, and concept
identity is already an open question in the knowledge model — this makes it
load-bearing rather than theoretical.

**Consolidation.** Relations derived from co-retrieval — these two concepts keep
coming back together — are derived edges, and the rule travels with them. They
are not the same kind of edge as an authored link, and a graph that cannot tell
them apart has laundered a statistic into a claim.

**Forgetting.** Knowledge retains conflicting claims by design; a memory that
never evicts is a memory that gets slower forever. The tension is real and is not
resolved by picking one side: eviction has to be a recorded operation over
knowledge, not a silent drop, and recency or salience are ranking inputs rather
than statements about truth.

## Implementation path

Ordered so each step is testable without the one after it.

1. **Ordering primitives** — *done.* `Seq`, `take(n)`, and a tie rule. The
   prefix question resolved better than this document expected: a `Card` is
   orderable by its own name, so a prefix is reproducible without vault order
   becoming observable. See [orderable cards](../../memory/orderable-cards.md).
2. **`Hit`, `Ranking`, `Retrieval`** — *done.* `semantic` is a pure function
   over a supplied corpus with a locally computed embedding, `hql.hashbag.v1`,
   deterministic and exact. Every hit carries the rule that produced it.
3. **The index extension** — *partly.* Embeddings are computed per query rather
   than stored, and the card's metadata carries only what index it belongs to,
   under `metadata.search`, a key the extension owns. Index revision and horizon
   are not modelled.
4. **The execution contract** — *not started*, and it is the one that cannot be
   deferred past a real store. Nothing is approximate yet, which is exactly why
   `approximate` is already a field: the day it is true, every consumer can see
   it.
5. **Traversal** — *done.* `expand(depth, direction)` in the graph domain, with
   the relation-card seeding rule and per-node presence provenance.
6. **`recall` over knowledge** — *not started.* Cardless concepts remain
   unreachable, and the implementation says so rather than pretending `semantic`
   covers them.
7. **The write path** — *not started.* Effects, reconciliation, consolidation,
   eviction.

Steps one and two were language work and needed no infrastructure, which is why
they came first. Step four remains the one that cannot be deferred past a real
index, because implementing the index first and describing its semantics
afterwards is how approximation becomes an undocumented property of the
language.

## Open questions

- **Whether this is a domain or a capability.** It is written here as a third
  model document because its vocabulary has no other home, while it deliberately
  mints no node and no edge. If that is wrong, the correction is that `Hit` and
  `Ranking` belong to the knowledge domain as evidence types.
- **`Query` construction** and whether a bare string literal may stand for one.
- **Whether `Ranking[T] <: Seq[Hit[T]]`** or ranking is a carrier that yields a
  sequence, which is the same shape as the graph-materialization question.
- **Whether a score may ever be persisted**, for example as a cached answer to a
  pinned query against a pinned index revision.
- **Chunking.** A card is not a retrieval unit; a passage is. What a `Hit[Card]`
  means when the match was one section, and whether `Hit[Section]` exists.
- **Multiple indexes.** Which answers, how results from two models combine, and
  whether combining is a ranking operation or a knowledge one.
- **Hybrid retrieval.** Lexical and structural signals beside the vector one, and
  whether a fused score can still carry an honest `Retrieval`.
- **Filtering before or after.** `cards | typed ConceptCard | semantic(q)` versus
  filtering the ranking, which differ in what the index must support.
- **Whether `expand` and `graph` are two functions or one**, which is the naming
  question [graphs.md](graphs.md) records, reached from a second direction.

See [knowledge model](knowledge.md) for the claims a hit produces,
[graph model](graphs.md) for the traversal this depends on,
[Card](../../wiki/hql/types/card-type.hmd) for the assembled metadata an index
contributes to, [fields](../../wiki/hql/fields.hmd) for the two metadata layers a
ranking must not confuse, [collections](../../wiki/hql/collections.hmd) for
ordering and [pipes](../../wiki/hql/types/pipes.hmd) for the execution freedom
this model constrains.
