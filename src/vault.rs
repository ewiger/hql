//! Loading a vault: documents on a filesystem, the cards built from them, and
//! the document graph their references write.
//!
//! An extension owns type knowledge and operations, never the data. This
//! module coordinates source, document, knowledge, and extension assembly.

use crate::document::Card;
use crate::extensions::Config;
use crate::extensions::semantic::Semantics;
use crate::graph::Edge;
use crate::warnings::Warning;
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
    pub warnings: Vec<String>,
    /// What the vault's `hql.toml` says about extensions.
    pub extensions: Config,
    /// What it says about its embedding index.
    pub semantics: Semantics,
}

impl Vault {
    /// An empty vault, which is what a program without `--vault` gets.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            root: PathBuf::new(),
            cards: Vec::new(),
            by_name: BTreeMap::new(),
            edges: Vec::new(),
            warnings: Vec::new(),
            extensions: Config::default(),
            semantics: Semantics::default(),
        }
    }

    /// Whether a vault was supplied at all.
    #[must_use]
    pub fn is_present(&self) -> bool {
        !self.root.as_os_str().is_empty()
    }

    /// The name this vault reports as the index a retrieval used.
    #[must_use]
    pub fn index_name(&self) -> String {
        if self.is_present() {
            self.root.display().to_string()
        } else {
            "<no vault>".to_owned()
        }
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

/// Load every Markdown and HyperMarkDown document under a directory.
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read.
pub fn load(root: &Path) -> io::Result<Vault> {
    load_source(&crate::sources::Filesystem::new(root))
}

/// Load a source through document and knowledge assembly.
///
/// # Errors
/// Returns a source read failure before assembly starts.
pub fn load_source(source: &dyn crate::sources::VaultSource) -> io::Result<Vault> {
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
        extensions: source.extensions(),
        semantics: source.semantics(),
        root,
        cards,
        by_name,
        edges,
        warnings,
    })
}
/// Turn loader notes into warnings a caller can report.
#[must_use]
pub fn warnings(vault: &Vault, span: std::ops::Range<usize>) -> Vec<Warning> {
    vault
        .warnings
        .iter()
        .map(|message| Warning::new(span.clone(), message.clone()))
        .collect()
}
