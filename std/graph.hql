// The graph domain: structural, and it knows nothing about meaning.
//
// See doc/models/domain/graphs.md and doc/wiki/hql/types/graph-type.hmd.
//
// There is no Node type. What a node is arrives as a type parameter, so a graph
// over documents and a graph over concepts share one declaration.

type Edge<S, T> {
    source: S
    target: T
    data:   Data
}

type Graph<N, E>

// The document graph: authored links over cards.
type Link<S, T> : Edge<S, T>
type HmdGraph   : Graph<Card, Link>
