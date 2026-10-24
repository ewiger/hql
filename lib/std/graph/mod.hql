// The graph domain: structural, and it knows nothing about meaning.
//
// See doc/models/domain/graphs.md and doc/wiki/hql/types/graph-type.hmd.
//
// There is no Node type. What a node is arrives as a type parameter, so a graph
// over documents and a graph over concepts share one declaration.
module graph
import core
import doc
import retrieval

// The carriers are not yet generic: whatever S and T are, an endpoint is held
// as the name of the node it is. A graph is over the vault's cards, so its
// nodes are cards and its edges the links among them. They are sets, because
// discovering a graph does not order it.
type Edge<S, T> {
    source : String
    target : String
    data   : Data
}

type Graph<N, E> {
    nodes : Set<Card>
    edges : Set<Edge>
}

// The document graph: authored links over cards.
type Link<S, T> : Edge<S, T>
type HmdGraph   : Graph<Card, Link>

// A graph is projected from what names its nodes and edges: cards, hits over
// cards, the edges among them, or a graph, which projects to itself. One card
// or one hit is the graph of what it reaches.
fn Graph(xs: Graph) : Graph
fn Graph(xs: Collection<Doc>) : Graph
fn Graph(xs: Collection<Hit<Doc>>) : Graph
fn Graph(xs: Collection<Edge>) : Graph
fn Graph(x: Doc) : Graph
fn Graph(x: Hit<Doc>) : Graph
fn Graph(nodes: Collection<Doc>, edges: Collection<Edge>) : Graph
