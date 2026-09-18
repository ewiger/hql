//! Edges and graphs: the structural domain, which knows nothing about meaning.

use crate::data::Data;
use std::collections::BTreeMap;

/// Which edge set an edge belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// An authored link in a document graph.
    Link,
    /// A knowledge relation, which carries its own identity.
    Relation,
}

/// A directed connection with a source, a target and an open annotation.
#[derive(Debug, Clone)]
pub struct Edge {
    /// The name the edge leaves.
    pub source: String,
    /// The name the edge enters.
    pub target: String,
    /// The annotation, whose keys belong to whoever wrote them.
    pub data: Data,
    /// Which edge set it belongs to.
    pub kind: EdgeKind,
}

impl Edge {
    /// A link edge with no annotation.
    #[must_use]
    pub fn link(source: &str, target: &str, data: Data) -> Self {
        Self {
            source: source.to_owned(),
            target: target.to_owned(),
            data,
            kind: EdgeKind::Link,
        }
    }
}

/// Why a node is in a graph.
///
/// A subgraph built by retrieving and then expanding contains nodes nobody
/// scored. Without this, drawing it would assert that every node matched.
#[derive(Debug, Clone, PartialEq)]
pub enum Presence {
    /// The node was in the collection the graph was projected from.
    Member,
    /// The node was retrieved, with the score that retrieved it.
    Retrieved(f64),
    /// The node was reached by expansion, from a seed, at a distance.
    Expanded { seed: String, depth: usize },
}

impl Presence {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Member => "member".to_owned(),
            Self::Retrieved(score) => format!("retrieved {score:.4}"),
            Self::Expanded { seed, depth } => format!("expanded from {seed} at depth {depth}"),
        }
    }
}

/// A set of nodes and a set of edges over them.
#[derive(Debug, Clone)]
pub struct Graph {
    /// Node names, kept sorted so rendering is deterministic.
    pub nodes: Vec<String>,
    /// The edges, in the order they were collected.
    pub edges: Vec<Edge>,
    /// Why each node is present.
    pub presence: BTreeMap<String, Presence>,
}

impl Graph {
    /// Build a graph, sorting nodes and dropping duplicates.
    #[must_use]
    pub fn new(presence: BTreeMap<String, Presence>, edges: Vec<Edge>) -> Self {
        let nodes: Vec<String> = presence.keys().cloned().collect();
        Self {
            nodes,
            edges,
            presence,
        }
    }

    /// The edges whose endpoints are both nodes of this graph.
    #[must_use]
    pub fn induced(&self) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| {
                self.presence.contains_key(&edge.source) && self.presence.contains_key(&edge.target)
            })
            .collect()
    }
}
