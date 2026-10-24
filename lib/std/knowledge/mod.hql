// Module `knowledge`: the knowledge domain, semantic, where everything is a
// claim. Every declaration is a type; the module declares no operation.
//
// See doc/models/domain/knowledge.md and doc/wiki/hql/knowledge.hmd.
module knowledge
import core
import doc
import graph

abstract type Concept
abstract type Proposition
abstract type Evidence
abstract type Knowledge

// A relation narrows the edge: source, target and data, plus identity,
// evidence and provenance. It is never a Card.
type Relation<S, T> : Edge<S, T>

// The two kinds of card, which say what the document is about.
type ConceptCard  : {Card, Concept} // its subject is a concept — a node
type RelationCard : Card           // its subject is a relation — an edge

// Both contracts at once, so the projection loses nothing: a knowledge graph is
// a graph and is still knowledge.
type KnowledgeGraph : {Knowledge, Graph<Concept, Relation>}
