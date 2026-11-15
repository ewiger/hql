//! What a run is checked and evaluated against: the configuration of the HQL
//! project, and the HyperMarkdown vault when the run has one.
//!
//! A project is not a vault. The two are found by separate walks, and may be
//! one directory or two (HQL-0059), so a run holds each apart: either may be
//! absent without the other.

use crate::diagnostics::Diagnostic;
use crate::document::Card;
use crate::endpoints::Endpoints;
use crate::extensions::Config;
use crate::extensions::format::Renderers;
use crate::graph::Edge;
use crate::vault::Vault;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What a program is checked and run against: the project's configuration,
/// and the vault when the run has one.
#[derive(Debug, Clone, Default)]
pub struct RunEnvironment {
    /// The root of the HQL project the run is configured by; `None` when the
    /// run has no project.
    pub project: Option<PathBuf>,
    /// What every configuration layer says about modules and packages.
    pub extensions: Config,
    /// Which renderer writes a type in a presentation format, from the
    /// project.
    pub formats: Renderers,
    /// The endpoints the project's `.hql/config.toml` configures, by id.
    pub endpoints: Endpoints,
    /// The vault whose documents are the run's cards; `None` when the run
    /// has no vault.
    pub vault: Option<Vault>,
}

impl RunEnvironment {
    /// A run with no project and no vault, under the default configuration.
    #[must_use]
    pub fn bare() -> Self {
        Self::default()
    }

    /// Whether the run has a vault.
    #[must_use]
    pub fn has_vault(&self) -> bool {
        self.vault.is_some()
    }

    /// Every card of the vault, in canonical name order; none without a
    /// vault.
    #[must_use]
    pub fn cards(&self) -> &[Arc<Card>] {
        self.vault.as_ref().map_or(&[], |vault| &vault.cards)
    }

    /// Resolve a document reference by name in the vault.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<Card>> {
        self.vault.as_ref()?.resolve(name)
    }

    /// The edges leaving a node of the vault.
    #[must_use]
    pub fn uplinks(&self, name: &str) -> Vec<&Edge> {
        self.vault
            .as_ref()
            .map_or_else(Vec::new, |vault| vault.uplinks(name))
    }

    /// The edges entering a node of the vault.
    #[must_use]
    pub fn downlinks(&self, name: &str) -> Vec<&Edge> {
        self.vault
            .as_ref()
            .map_or_else(Vec::new, |vault| vault.downlinks(name))
    }

    /// What loading the vault wanted to say without refusing to load.
    #[must_use]
    pub fn warnings(&self) -> &[Diagnostic] {
        self.vault.as_ref().map_or(&[], |vault| &vault.warnings)
    }

    /// The name a retrieval reports as the index it used: the vault's, or a
    /// placeholder without one.
    #[must_use]
    pub fn index_name(&self) -> String {
        self.vault
            .as_ref()
            .map_or_else(|| "<no vault>".to_owned(), Vault::index_name)
    }

    /// The project root, when the run has a project.
    #[must_use]
    pub fn project_root(&self) -> Option<&Path> {
        self.project.as_deref()
    }

    /// Make `hmd` active for every program of this run, as a HyperMarkdown
    /// host does by injecting `import hmd`.
    pub fn host(&mut self) {
        if !self.extensions.hmd_active() {
            self.extensions.imports.push("hmd".to_owned());
        }
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
        self.formats = Renderers::read(root).map_err(invalid)?;
        self.endpoints = Endpoints::read(root).map_err(invalid)?;
        self.project = Some(root.to_path_buf());
        Ok(())
    }
}

/// Read a directory as a HyperMarkdown host that names it does: its
/// documents are the cards of the run's vault whether or not it holds
/// `.hmd/`; when it is also a project, holding `.hql/config.toml`, the run
/// takes that project's configuration; and `hmd` is active, as a host that
/// supplies a vault injects `import hmd` (HQL-0059).
///
/// # Errors
///
/// Returns the underlying I/O failure when the root cannot be read, and a
/// malformed project configuration.
pub fn load(root: &Path) -> io::Result<RunEnvironment> {
    let mut env = RunEnvironment {
        vault: Some(crate::vault::load_documents(root)?),
        ..RunEnvironment::default()
    };
    if crate::context::is_project(root) {
        env.configure_project(root)?;
    }
    env.host();
    Ok(env)
}

/// Load a source through document and knowledge assembly, under the
/// configuration the source supplies.
///
/// # Errors
/// Returns a source read failure before assembly starts, and a malformed
/// renderer or endpoint entry of the source's configuration.
pub fn load_source(source: &dyn crate::sources::VaultSource) -> io::Result<RunEnvironment> {
    let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
    let vault = crate::vault::assemble(source)?;
    Ok(RunEnvironment {
        project: None,
        extensions: source.extensions(),
        formats: source.formats().map_err(invalid)?,
        endpoints: source.endpoints().map_err(invalid)?,
        vault: Some(vault),
    })
}
