// See doc/models/domain/knowledge.md and doc/wiki/hql/knowledge.hmd.

/// Knowledge: the semantic domain, where everything is a claim. Every
/// declaration is a type; the module declares no operation.
module knowledge

import core
import doc
import graph

/// Something identified or described: the type that puts a card in a node
/// position.
abstract type Concept

/// A claim that can be supported, disputed or constrained.
abstract type Proposition

/// Material offered for or against a claim.
abstract type Evidence

/// The universe of concepts, relations, propositions and evidence.
abstract type Knowledge

/// A meaningful relation from `source` to `target`: an `Edge` narrowed with
/// identity, evidence and provenance. It is never a `Card`.
type Relation<S, T> : Edge<S, T>

/// A card whose subject is a concept: a `Card` that is a `Concept`, and so
/// a node. Every card that is not a `RelationCard` is one.
type ConceptCard  : {Card, Concept} // its subject is a concept — a node

/// A card whose subject is a relation: one whose `metadata` declares
/// `knowledge.type: Relation`, with a `knowledge.relation.source` and
/// `target` that each name a card of the vault. A graph projected from
/// cards takes its source and target as nodes, not the card itself.
type RelationCard : Card           // its subject is a relation — an edge

/// A graph whose nodes are concepts and whose edges are relations, which is
/// still `Knowledge`: the projection loses neither contract.
type KnowledgeGraph : {Knowledge, Graph<Concept, Relation>}
