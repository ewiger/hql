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
    /// The vault root, holding `.hmd/`; also the index name a retrieval
    /// reports.
    pub root: PathBuf,
    /// The namespace root, where the cards start: a document's path is
    /// relative to it.
    pub namespace: PathBuf,
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
    /// HyperMarkdown's resolver over the same cards: every reference resolves
    /// through it, so HQL and HyperMarkdown name the same card.
    resolver: Option<Arc<hypermarkdown::Vault>>,
    /// A card's path, written with `/`, to its name.
    by_path: BTreeMap<String, String>,
}

impl Vault {
    /// The name this vault reports as the index a retrieval used.
    #[must_use]
    pub fn index_name(&self) -> String {
        self.root.display().to_string()
    }

    /// Where a document of this vault is on disk: the namespace root joined
    /// with its path, which is its place in the vault.
    #[must_use]
    pub fn file_of(&self, document: &crate::document::Document) -> PathBuf {
        self.namespace.join(&document.path)
    }

    /// The card a name names.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<Card>> {
        self.by_name.get(name).map(Arc::clone)
    }

    /// The card a reference written in a program names, `@[[owl]]`, resolved
    /// as HyperMarkdown resolves a reference written at the namespace root.
    ///
    /// # Errors
    /// The reference is ambiguous: its candidates, by path.
    pub fn reference(&self, written: &str) -> Result<Option<Arc<Card>>, Vec<String>> {
        match self.lookup("", written) {
            Lookup::Card(name) => Ok(self.resolve(&name)),
            Lookup::Ambiguous(candidates) => Err(candidates),
            Lookup::None => Ok(None),
        }
    }

    /// What a reference written in the card at `source` names.
    fn lookup(&self, source: &str, written: &str) -> Lookup {
        let Some(resolver) = &self.resolver else {
            return if self.by_name.contains_key(written) {
                Lookup::Card(written.to_owned())
            } else {
                Lookup::None
            };
        };
        lookup(resolver, &self.by_path, source, written)
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

/// Read every Markdown and HyperMarkdown document under a vault's namespace
/// root as its cards, with the vault's own index configuration.
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read, and a
/// malformed `.hmd/config.toml`.
pub fn load_documents(root: &Path) -> io::Result<Vault> {
    assemble(&crate::sources::Filesystem::new(root)?)
}

/// Assemble the vault of a source: its documents as cards, their edges, and
/// its index configuration.
///
/// # Errors
/// Returns a source read failure before assembly starts.
pub fn assemble(source: &dyn crate::sources::VaultSource) -> io::Result<Vault> {
    let snapshot = source.read()?;
    let root = snapshot.root.clone();
    let namespace = source.namespace().unwrap_or_else(|| root.clone());
    // HyperMarkdown's resolver over exactly the documents HQL reads. The
    // vault's configuration steers it: autodiscovery and its mode.
    let config = if is_root(&root) {
        hypermarkdown::Config::read(&root)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?
    } else {
        hypermarkdown::Config::default()
    };
    let host = hypermarkdown::MemoryHost::new(snapshot.documents.iter().filter_map(|document| {
        let text = document.text.as_ref().ok()?;
        Some((crate::document::posix(&document.path), text.clone()))
    }));
    let resolver = hypermarkdown::Vault::load(&host, config)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let mut store = crate::document::DocumentStore::from_source(snapshot);
    let by_path: BTreeMap<String, String> = store
        .documents
        .iter()
        .map(|document| {
            (
                crate::document::posix(&document.path),
                document.name.clone(),
            )
        })
        .collect();
    // Every reference a card writes names the card HyperMarkdown resolves it
    // to from that card. One that resolves to no card stays as written, a
    // forward reference; an ambiguous one names none and is reported.
    for document in &mut store.documents {
        let source = crate::document::posix(&document.path);
        for reference in &mut document.references {
            match lookup(&resolver, &by_path, &source, &reference.target) {
                Lookup::Card(name) => reference.target = name,
                Lookup::Ambiguous(candidates) => {
                    let mut warning = crate::diagnostics::Diagnostic::warning(
                        crate::diagnostics::Phase::Load,
                        crate::diagnostics::Kind::UnresolvedLink,
                        0..0,
                        format!(
                            "[[{}]] matches {} cards; qualify it (candidates: {})",
                            reference.target,
                            candidates.len(),
                            candidates.join(", ")
                        ),
                    );
                    warning.origin = Some(source.clone());
                    store.warnings.push(warning);
                }
                Lookup::None => {}
            }
        }
    }
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
        namespace,
        resolver: Some(Arc::new(resolver)),
        by_path,
        cards,
        by_name,
        edges,
        warnings,
    })
}

/// What a reference names.
enum Lookup {
    /// The card of this name.
    Card(String),
    /// More than one card, by path; HyperMarkdown chooses none.
    Ambiguous(Vec<String>),
    /// No card: a forward reference.
    None,
}

/// Resolve `written`, the text between `[[` and `]]`, from the card at
/// `source` as HyperMarkdown resolves it, and name the card it lands on.
fn lookup(
    resolver: &hypermarkdown::Vault,
    by_path: &BTreeMap<String, String>,
    source: &str,
    written: &str,
) -> Lookup {
    let page = hypermarkdown::Reference::parse(written)
        .map_or_else(|_| written.trim().to_owned(), |reference| reference.page);
    match resolver.resolve(source, &page) {
        hypermarkdown::Resolution::Resolved { path, .. } => by_path
            .get(&path)
            .cloned()
            .map_or(Lookup::None, Lookup::Card),
        hypermarkdown::Resolution::Ambiguous { candidates } => Lookup::Ambiguous(candidates),
        _ => Lookup::None,
    }
}
