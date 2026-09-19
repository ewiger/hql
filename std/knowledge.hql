// The knowledge domain: semantic, and everything in it is a claim.
//
// See doc/models/domain/knowledge.md and doc/wiki/hql/knowledge.hmd.

abstract type Concept
abstract type Proposition
abstract type Evidence
abstract type Knowledge

// A relation narrows the edge: source, target and data, plus identity,
// evidence and provenance. It is never a Card.
type Relation<S, T> : Edge<S, T>

// Card is the base representation type: knowledge given a physical document.
// Its two kinds say what the document is about.
abstract type Card : Doc {
    metadata : Data     // the knowledge attached to this card, assembled
}

type ConceptCard  : {Card, Concept} // its subject is a concept — a node
type RelationCard : Card           // its subject is a relation — an edge

// Card is abstract in the other direction too: a concrete card is one whose
// body is written in a given dialect, one subtype per `header.format`.
type HmdCard : {Card, HmdDoc} // format is hmd, so the body is HmdContent
type MdCard  : Card           // format is md, which no dialect type narrows

// Both contracts at once, so the projection loses nothing: a knowledge graph is
// a graph and is still knowledge.
type KnowledgeGraph : {Knowledge, Graph<Concept, Relation>}
