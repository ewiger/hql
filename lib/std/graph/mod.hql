// See doc/models/domain/graphs.md and doc/wiki/hql/types/graph-type.hmd.

/// Graphs: nodes and the edges among them, structural only; what a node or
/// an edge means is the `knowledge` module's. There is no `Node` type: what
/// a node is arrives as a type parameter, so a graph over documents and a
/// graph over concepts share one declaration.
module graph

import core
import doc
import retrieval

// The carriers are not yet generic: whatever S and T are, an endpoint is held
// as the name of the node it is.

/// An edge from `source` to `target`, with `data` about it. Whatever `S`
/// and `T` are, each end is held as the name of the node it is.
type Edge<S, T> {
    source : String
    target : String
    data   : Data
}

/// A graph: the set of its `nodes`, which are cards, and the set of its
/// `edges`, the links among them. Both are sets: discovering a graph does
/// not order it. `vault.graph` is every card of the vault and the links
/// among them.
type Graph<N, E> {
    nodes : Set<Card>
    edges : Set<Edge>
}

/// An authored link from one document to another: an `Edge` between cards.
type Link<S, T> : Edge<S, T>

/// The document graph: a `Graph` of cards and the authored `Link`s among
/// them.
type HmdGraph   : Graph<Card, Link>

/// `xs` itself: a graph projects to itself.
fn Graph(xs: Graph) : Graph

/// The graph of the cards in `xs`, or the cards its hits hold, as nodes,
/// with the vault's edges among them. A relation card is not a node: its
/// source and target are. `Graph(vault.cards)` has the nodes and edges of
/// `vault.graph`.
fn Graph(xs: Collection<Doc>) : Graph
fn Graph(xs: Collection<Hit<Doc>>) : Graph

/// The graph of the edges `xs`: their sources and targets as nodes, and the
/// edges as given.
fn Graph(xs: Collection<Edge>) : Graph

/// The graph of the one card `x`, or of the card the hit `x` holds: that
/// card as its only node, with any vault edge from it to itself.
fn Graph(x: Doc) : Graph
fn Graph(x: Hit<Doc>) : Graph

/// The graph of `nodes`, together with the source and target of every edge
/// in `edges`, and the edges as given.
fn Graph(nodes: Collection<Doc>, edges: Collection<Edge>) : Graph

/// The edges of the links `x` writes to other cards: one `Edge` per
/// authored link, its `source` the card `x`. Over a hit, the card it holds;
/// over an absent card, none; over a collection, every element's, in order.
fn uplinks(x: Option<Doc>) : List<Edge>
fn uplinks(x: Doc) : List<Edge>
fn uplinks(x: Hit<Card>) : List<Edge>
fn uplinks(xs: Collection<Doc>) : List<Edge>
fn uplinks(xs: Collection<Hit<Card>>) : List<Edge>

/// The edges of the links other cards of the vault write to `x`: one `Edge`
/// per authored link, its `target` the card `x`. Over a hit, the card it
/// holds; over an absent card, none; over a collection, every element's, in
/// order. It needs a vault: a document alone has nothing that points at it.
fn downlinks(x: Option<Doc>) : List<Edge>
fn downlinks(x: Doc) : List<Edge>
fn downlinks(x: Hit<Card>) : List<Edge>
fn downlinks(xs: Collection<Doc>) : List<Edge>
fn downlinks(xs: Collection<Hit<Card>>) : List<Edge>

// The form over an absent card comes first: a step whose first form takes
// an `Option` sees absence itself, as the native signature did, rather than
// being lifted over it.
/// The graph reached from `x` by following links `depth` steps, `1` unless
/// given, in `direction`: `"uplinks"`, `"downlinks"` or `"both"`, the
/// default. A depth of `0` is a type error when it is a literal and an
/// evaluation failure when it is computed: `Graph(xs)` is the graph with no
/// traversal. It needs a vault.
fn expand(x: Option<Doc>, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(x: Graph, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(x: Doc, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(xs: Collection<Doc>, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(xs: Collection<Hit<Card>>, depth: Int = 1, *, direction: String = "both") : Graph
