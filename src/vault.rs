//! Loading a vault: documents on a filesystem, the cards built from them, and
//! the document graph their references write.
//!
//! An extension owns type knowledge and operations, never the data. This
//! module coordinates source, document and knowledge assembly. A vault is
//! HyperMarkdown's and holds nothing of an HQL project: the project's
//! configuration is the [`crate::environment::RunEnvironment`]'s (HQL-0059).

use crate::document::Card;
use crate::extensions::semantic::Semantics;
use crate::graph::Edge;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A namespace of documents, the cards built from them, and their edges.
#[derive(Debug, Clone)]
pub struct Vault {
    /// Where the vault was loaded from; also the index name a retrieval reports.
    pub root: PathBuf,
    /// Every card, in canonical name order.
    pub cards: Vec<Arc<Card>>,
    /// Lookup by name.
    pub by_name: BTreeMap<String, Arc<Card>>,
    /// Link and relation edges over the vault.
    pub edges: Vec<Edge>,
    /// What loading wanted to say without refusing to load.
    pub warnings: Vec<crate::diagnostics::Diagnostic>,
    /// What the vault's `.hmd/config.toml` says about its embedding index.
    pub semantics: Semantics,
}

impl Vault {
    /// The name this vault reports as the index a retrieval used.
    #[must_use]
    pub fn index_name(&self) -> String {
        self.root.display().to_string()
    }

    /// Where a document of this vault is on disk: the root joined with its
    /// path, which is its place in the vault.
    #[must_use]
    pub fn file_of(&self, document: &crate::document::Document) -> PathBuf {
        self.root.join(&document.path)
    }

    /// Resolve a document reference by name.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<Card>> {
        self.by_name.get(name).map(Arc::clone)
    }

    /// The edges leaving a node.
    #[must_use]
    pub fn uplinks(&self, name: &str) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| edge.source == name)
            .collect()
    }

    /// The edges entering a node, which a document cannot know by itself.
    #[must_use]
    pub fn downlinks(&self, name: &str) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|edge| edge.target == name)
            .collect()
    }
}

/// The nearest directory at or above `start` that is a vault root, if any.
///
/// The walk is HyperMarkdown's: it stops at the first directory holding
/// `.hmd/`. A project's `.hql/` marks no vault.
#[must_use]
pub fn discover(start: &Path) -> Option<PathBuf> {
    crate::context::discover_vault(start)
}

/// Whether a directory marks a vault root: it holds `.hmd/`.
#[must_use]
pub fn is_root(dir: &Path) -> bool {
    crate::context::is_vault(dir)
}

/// Read every Markdown and HyperMarkdown document under a directory as the
/// cards of a vault, with the vault's own index configuration.
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read.
pub fn load_documents(root: &Path) -> io::Result<Vault> {
    assemble(&crate::sources::Filesystem::new(root))
}

/// Assemble the vault of a source: its documents as cards, their edges, and
/// its index configuration.
///
/// # Errors
/// Returns a source read failure before assembly starts.
pub fn assemble(source: &dyn crate::sources::VaultSource) -> io::Result<Vault> {
    let snapshot = source.read()?;
    let root = snapshot.root.clone();
    let store = crate::document::DocumentStore::from_source(snapshot);
    let names = store
        .documents
        .iter()
        .map(|document| document.name.clone())
        .collect();
    let mut warnings = store.warnings;
    let mut cards: Vec<_> = store
        .documents
        .into_iter()
        .map(|document| {
            let mut card = crate::knowledge::build_card(document, &names, &mut warnings);
            crate::extensions::contribute_metadata(&mut card);
            Arc::new(card)
        })
        .collect();
    cards.sort_by(|left, right| left.name().cmp(right.name()));
    let by_name = cards
        .iter()
        .map(|card| (card.name().to_owned(), Arc::clone(card)))
        .collect();
    let edges = crate::knowledge::build_edges(&cards);
    Ok(Vault {
        semantics: source.semantics(),
        root,
        cards,
        by_name,
        edges,
        warnings,
    })
}
