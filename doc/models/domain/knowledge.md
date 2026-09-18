# The knowledge domain

Status: preferred conceptual model, designed from day one rather than deferred.
The *implementation* is a bootstrap evaluator over Int, Float, Bool and same-type
addition, so nothing here is executable yet. Type syntax is illustrative.

This document defines one of the two domains HQL reasons in. The other is the
[graph model](graphs.md). They are peers rather than layers, and a type in one
is never a supertype of a type in the other — see
[type system](../../wiki/hql/type-system.hmd). The graph domain is structural:
nodes, edges, adjacency. This one is semantic, and everything in it is a claim
that someone or something made.

**`Card` is the base representation type of this domain**: a document that gives
a piece of knowledge a physical form. It has two kinds. A `ConceptCard` is also a
`Concept` and occupies a node position. A `RelationCard` clarifies a relation,
which is an edge. Most knowledge lives in nodes, so the first is the ordinary
case and the second is the exception.

## Vocabulary

```text
Concept       something identified or described
Relation      a meaningful relationship with typed endpoints and identity
Proposition   a claim that can be supported, disputed or constrained
Evidence      material offered for or against a claim
Knowledge     the universe composed of all of these

Card          the base representation: knowledge given a physical document
ConceptCard   a card whose subject is a concept — a node
RelationCard  a card whose subject is a relation — an edge, clarified
```

The first block is what the domain is *about*. The second is how it is written
down. Keeping the two apart is what the rest of this document turns on: a card
is a representation of something, and never the thing it represents.

`Knowledge` is the working name of the type. It is richer than a graph and
richer than a collection of cards: it combines document structure, headers, card
metadata, concepts, relations, imported knowledge and explicit derivations,
and it retains what those disagree about.

## Interpretation is a step, not a reading

Nothing in this domain is free. A link is structural connectivity and asserts no
semantic relation. A `type: WorksOn` entry in a header is an author's claim, not
a proof of conformance. A folder is not a taxonomy, and a tag is not a concept.

The path from document to knowledge therefore runs through an explicit
interpretation, and **the rule used must be retained with the result**. A
derivation that cannot say how it was derived has produced an assertion, not
knowledge. This is the single rule this domain is built on; most of what follows
is a consequence of it.

## Card is the base representation

A [card](../../wiki/hql/types/card-type.hmd) gives a piece of knowledge a
physical representation. That is all `Card` says, and saying only that is the
point: the card is the document, and what the document is *about* is declared by
its two subtypes.

```hql
type Card <: Doc                      // knowledge, written down

type ConceptCard  <: {Card, Concept}  // its subject is a concept
type RelationCard <: Card             // its subject is a relation
```

`ConceptCard <: {Card, Concept}` is the normalized form of
`{Card, Concept, Doc}`: `Card <: Doc` already holds, so naming `Doc` again adds
nothing — see [type system](../../wiki/hql/type-system.hmd) for the
normalization rule. Read it as the declaration it is: a concept card is a
document, a card, and a concept, and it is `Concept` that puts it in a node
position.

Card-ness is a knowledge notion, independent of file format: Markdown, HMD or
structured data can supply the representation equally well.

### Which kind a card is

The kind is declared in the card's knowledge layer, not inferred from the file:

```hql
card.metadata.knowledge.type : KnowledgeKind   // Concept | Relation
```

`Concept` is the default. A card that declares nothing is a `ConceptCard`,
because a vault of notes is overwhelmingly a vault of concepts.
`metadata.knowledge.type == Relation` is the explicit opt-in that makes a card a
`RelationCard`.

Like every other refinement, the entry is a claim and has to be checked. An
authored `knowledge.type: Relation` does not by itself produce a `RelationCard`;
the contract — endpoints that resolve, an identity, a relation to represent — is
what produces one. An unchecked entry would let any authored key mint a type.

## ConceptCard: the ordinary case

A concept card occupies a node position in a knowledge graph. It can describe
the concept, carry propositions about it and supply evidence for those claims.

| Knowledge element | Graph role |
| --- | --- |
| `ConceptCard`, which is a `Concept` | Node |
| `Relation` between concepts | Edge between nodes |

An Alice card and a Project Atlas card are two concept nodes. A `WorksOn`
relation connects them. Concepts imported without any card occupy node positions
too — `ConceptCard <: Concept` does not require every concept to have a card,
which is why `Graph[Concept, Relation]` is parameterized by `Concept` rather
than by `ConceptCard`.

## RelationCard: the exception

Most knowledge lives in nodes. But a relation is sometimes the thing that needs
explaining — why Alice works on Project Atlas, on whose authority, against what
evidence — and a link annotation has nowhere to put that. A `RelationCard` is
the escape hatch: the edge is promoted into a document so it gains a body, a
name in the vault and incoming links of its own.

```hql
card.metadata.knowledge.type == Relation
card.metadata.relation : Relation[S, T]
```

**A `RelationCard` is not a `Relation`, and it is not a `ConceptCard` either.**
Representation is not ancestry. The card is a document about a relation; the
relation is a separate value, reached through the card's knowledge layer by
ordinary refinement. This is the whole reason `Card` is the base type: it is the
one thing both kinds genuinely are.

That the card is not a concept node is the substance of the exception. A
`RelationCard` contributes its relation to the edge set; it does not add a node
to the graph. Were it a node as well, every explained relationship would show up
twice — once as the edge it is and once as a node beside it — and the graph
would stop being a graph of concepts.

## Relation is a first-class type

`Relation` stands in the vocabulary beside `Concept`, `Proposition` and
`Evidence`. It has typed endpoints, its own identity, and evidence and
provenance behind it, and it acquires none of that from a document: a relation
may be imported from another namespace, derived by a rule, or read out of a
store with no card anywhere. `RelationCard` is one way a relation gets written
down, not what a relation is.

What a relation narrows is the edge:

```hql
type Relation[S, T] <: Edge[S, T]
```

An edge has source, target and data; a relation adds identity, evidence and
provenance. Because that identity is its own, the knowledge graph is a
multigraph and can hold two conflicting claims about one pair — see
[graph model](graphs.md).

Ordinary document links stay structural. They may carry data — see the
[link operator](../../wiki/hql/operators/link-op.hmd) — but interpreting one as
a semantic relation requires an explicit rule and a checked contract. Building
the knowledge graph therefore places concept cards as nodes and collects
relations as edges, including those a relation card represents.

## A card's two layers

Every card, of either kind, has a knowledge layer in addition to its document
header:

```hql
card.header   : Data   // what is known about the document
card.metadata : Data   // the knowledge attached to this card
```

`header` is document-level and every document has one. `metadata` is card-only
and knowledge-level, and it is a **different field** from the document's authored
`header.metadata` rather than that field renamed:

```hql
card.header.metadata?   // optional — what the author wrote, if anything
card.metadata           // always present — assembled when the card is built
```

The authored block is optional, because authoring metadata is a choice. The
card's layer is not, because it exists whether or not the file said anything: it
is assembled from the authored entries, system metadata, extension contributions,
vault state and other derived information, under precedence rules
[Card](../../wiki/hql/types/card-type.hmd) owns. Both paths stay addressable, and
a query has to know which layer it is asking about — see
[fields](../../wiki/hql/fields.hmd).

The knowledge extension then refines it:

```hql
import knowledge

card.metadata          : KnowledgeMetadata
card.metadata.concepts : List[Concept]
```

That is progressive refinement of one unchanged tree, not conversion. An
extension owns type knowledge about the tree, never the tree.

The concepts referenced in `card.metadata.concepts` do not replace a concept
card's own role as a node. Metadata describes the knowledge attached to that
node; its presence alone establishes neither the meaning nor the validity of a
claim.

## Conflict, provenance and truth

Distinct claims about the same subject are the normal case, not an error state.
Knowledge retains them with their identities intact. A merge that silently
collapses two claims into one has destroyed the disagreement, which was the
information worth keeping.

**Provenance records where a claim came from, and confers nothing.** A location,
an author, an import or a derivation rule describes origin; it does not make a
claim true, and a high-provenance claim does not outrank a low-provenance one by
being better sourced. Projections must not turn an assertion into a truth on the
way out.

## Constraints, evidence and satisfaction

A constraint states a requirement over knowledge — that a claim of a given kind
carries an Evidence section, that a relation's endpoints exist, that a concept is
defined once. Evidence can be ordinary document content: a section with links is
a perfectly good carrier.

Satisfaction is structural and says exactly that much. A constraint requiring an
Evidence section with links is satisfied by an Evidence section with links; it
does not prove the claim, assess the quality of its support, or make the cited
material relevant. Conflating structural satisfaction with truth would undo the
interpretation rule this domain rests on.

The type of a satisfaction result, how failures are carried, and whether
constraint checking is a pure function over knowledge or an effect are open.

## Projection and presentation

```hql
knowledge
| graph
```

A projection is a function from knowledge to some other value, and `graph` is
one of them. Tables, lists, trees and diagrams are others, and none is the
knowledge itself. A projection may lose information — a drawing keeps adjacency
and drops rationale — which is acceptable exactly as long as it is not mistaken
for a lossless view.

The graph projection is the one case where losing it is not acceptable, and the
goal is a type that says so:

```hql
type KnowledgeGraph <: {Knowledge, Graph[Concept, Relation]}
```

A knowledge graph is a graph — concepts, concept cards among them, as nodes and
relations as edges — and is still knowledge, retaining conflicting claims, provenance, evidence and
constraints. Both contracts hold at once, so the projection loses nothing and
the result can still be interrogated as knowledge rather than only drawn. A bare
`Graph[Concept, Relation]` remains the weaker, lossy result for a host that only
needs a picture. See [type system](../../wiki/hql/type-system.hmd) for what the
declaration demands and [graph model](graphs.md) for the structural half.

Presenters (`table`, `json`, `graph`, `markdown`, `tree`, `text`, `value`,
`empty`) produce renderable values and are terminal with respect to semantic
processing. The host renders. A host may pick a default presenter by result
type, but knowledge requires an explicit one, because its projections are
equally valid and choosing silently would be choosing a reading.

## Boundaries

Knowledge is designed from day one rather than deferred behind a release
boundary, and it depends on the graph capabilities rather than the reverse. The
layering is core → graph → knowledge, with document sources beside it. HMD is
one such source; nothing in this domain depends on it.

Namespace visibility and the merge rules for imported knowledge are open, and
`knowledge` does not imply unrestricted global access.

## Open questions

- Semantic constructors and derivation syntax, and how a retained rule is
  represented.
- Concept identity: when two concept cards, or a concept card and an imported
  concept, are the same concept.
- Whether a `RelationCard`'s own `downlinks` — the cards that point at it as a
  document — belong in the knowledge graph at all, given that the card is not a
  node in it.
- Whether `metadata.knowledge.type` admits kinds beyond `Concept` and `Relation`,
  and whether a card may change kind without changing identity.
- What a card's metadata holds before an extension declares its shape, and the
  precedence between its authored, system, extension and vault-state inputs.
- Satisfaction and validation types, and what a failed narrowing yields.
- Imported-knowledge merge policy and namespace visibility.
- Whether the projection and the view may share the name `graph`.
- Effects: constructing knowledge is pure, and contributing it to a store is
  not; the boundary has not been drawn.

See [knowledge](../../wiki/hql/knowledge.hmd) for the wiki card,
[Card](../../wiki/hql/types/card-type.hmd) for the document that carries
knowledge, [type system](../../wiki/hql/type-system.hmd) for narrowing and
refinement, [graph model](graphs.md) for the other domain,
[semantic search](semantic-search.md) for retrieval and the read path of AI
memory, and [program values](../behavior/program-values.md) for what a program
returns.
