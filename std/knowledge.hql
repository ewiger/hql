// The knowledge domain: semantic, and everything in it is a claim.
//
// See doc/models/domain/knowledge.md and doc/wiki/hql/knowledge.hmd.

type Concept
type Proposition
type Evidence
type Knowledge

// A relation narrows the edge: source, target and data, plus identity,
// evidence and provenance. It is never a Card.
type Relation<S, T> <: Edge<S, T>

// Card is the base representation type: knowledge given a physical document.
// Its two kinds say what the document is about.
type Card <: Doc {
    metadata : Data     // the knowledge attached to this card, assembled
}

type ConceptCard  <: {Card, Concept}   // its subject is a concept — a node
type RelationCard <: Card              // its subject is a relation — an edge

// Both contracts at once, so the projection loses nothing: a knowledge graph is
// a graph and is still knowledge.
type KnowledgeGraph <: {Knowledge, Graph<Concept, Relation>}
