//! Loading a vault: documents on a filesystem, the cards built from them, and
//! the document graph their references write.
//!
//! An extension owns type knowledge and operations, never the data. This
//! module coordinates source, document, knowledge, and extension assembly.

use crate::document::Card;
use crate::extensions::Config;
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
    /// The root of the HQL project the run is configured by; empty when the
    /// run has none. A project is not the vault: the two are found apart
    /// (HQL-0059), and may be one directory or two.
    pub project: PathBuf,
    /// What the project's `.hql/config.toml` says about modules and
    /// packages.
    pub extensions: Config,
    /// What the vault's `.hmd/config.toml` says about its embedding index.
    pub semantics: Semantics,
    /// Which renderer writes a type in a presentation format, from the
    /// project.
    pub formats: crate::extensions::format::Renderers,
    /// The endpoints the project's `.hql/config.toml` configures, by id.
    pub endpoints: crate::endpoints::Endpoints,
}

impl Vault {
    /// An empty vault, which is what a program outside every vault gets.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            root: PathBuf::new(),
            cards: Vec::new(),
            by_name: BTreeMap::new(),
            edges: Vec::new(),
            warnings: Vec::new(),
            project: PathBuf::new(),
            extensions: Config::default(),
            semantics: Semantics::default(),
            formats: crate::extensions::format::Renderers::default(),
            endpoints: crate::endpoints::Endpoints::default(),
        }
    }

    /// Whether a vault was supplied at all.
    #[must_use]
    pub fn is_present(&self) -> bool {
        !self.root.as_os_str().is_empty()
    }

    /// Make `hmd` active for every program run against this vault, as a
    /// HyperMarkdown host does by injecting `import hmd`.
    pub fn host(&mut self) {
        if !self.extensions.hmd_active() {
            self.extensions.imports.push("hmd".to_owned());
        }
    }

    /// The project root, when the run has a project.
    #[must_use]
    pub fn project_root(&self) -> Option<&Path> {
        (!self.project.as_os_str().is_empty()).then_some(self.project.as_path())
    }

    /// Take the configuration of the project at `root`: its packages,
    /// renderers and endpoints, each relative path read against `root`.
    ///
    /// # Errors
    ///
    /// A malformed renderer or endpoint entry, naming it.
    pub fn configure_project(&mut self, root: &Path) -> io::Result<()> {
        let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
        self.extensions = Config::read(root);
        self.formats = crate::extensions::format::Renderers::read(root).map_err(invalid)?;
        self.endpoints = crate::endpoints::Endpoints::read(root).map_err(invalid)?;
        self.project = root.to_path_buf();
        Ok(())
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
/// cards of a vault, with the vault's own index configuration, and nothing
/// of any project.
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read.
pub fn load_documents(root: &Path) -> io::Result<Vault> {
    load_source(&crate::sources::Filesystem::new(root))
}

/// Read a directory as a vault, as a HyperMarkdown host that names it does:
/// its documents are cards whether or not it holds `.hmd/`; when it is also a
/// project, holding `.hql/config.toml`, the run takes that project's
/// configuration; and `hmd` is active, as a host that supplies a vault
/// injects `import hmd` (HQL-0059).
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read, and a
/// malformed project configuration.
pub fn load(root: &Path) -> io::Result<Vault> {
    let mut vault = load_documents(root)?;
    if crate::context::is_project(root) {
        vault.configure_project(root)?;
    }
    vault.host();
    Ok(vault)
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
        project: PathBuf::new(),
        extensions: source.extensions(),
        semantics: source.semantics(),
        formats: source
            .formats()
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?,
        endpoints: source
            .endpoints()
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?,
        root,
        cards,
        by_name,
        edges,
        warnings,
    })
}
