//! How a vault's documents are read, independent of document parsing and
//! knowledge assembly. See `doc/proposals/HQL-0012/README.md`.

use crate::document::Format;
use std::io;
use std::path::{Path, PathBuf};

/// One document's identity, dialect, and source read result.
#[derive(Debug)]
pub struct SourceDocument {
    /// Source identity relative to the source root.
    pub path: PathBuf,
    /// The dialect the source supplies.
    pub format: Format,
    /// Source text, or a read error to report while loading other documents.
    pub text: io::Result<String>,
}

/// One snapshot of a source before names and document structure are assigned.
#[derive(Debug)]
pub struct Snapshot {
    /// Source identity, used for document paths and retrieval provenance.
    pub root: PathBuf,
    /// Documents in this snapshot; assembly does not depend on their order.
    pub documents: Vec<SourceDocument>,
}

/// A source supplies text and configuration without assembling knowledge.
pub trait VaultSource: Send + Sync {
    /// Read a source snapshot, failing if the source itself is unreadable.
    fn read(&self) -> io::Result<Snapshot>;
    /// Extensions enabled by this source.
    fn extensions(&self) -> crate::extensions::Config {
        crate::extensions::Config::default()
    }
    /// Semantic index configuration supplied by this source.
    fn semantics(&self) -> crate::extensions::semantic::Semantics {
        crate::extensions::semantic::Semantics::default()
    }
    /// Model, recording and memory configuration supplied by this source.
    fn ai(&self) -> crate::extensions::ai::transport::Settings {
        crate::extensions::ai::transport::Settings::default()
    }
    /// Renderer choices supplied by this source.
    ///
    /// # Errors
    /// Names a configured renderer this binary does not have.
    fn formats(&self) -> Result<crate::extensions::format::Renderers, String> {
        Ok(crate::extensions::format::Renderers::default())
    }
    /// The endpoints this source's configuration declares.
    ///
    /// # Errors
    /// Names the first malformed `[endpoint.<id>]` entry.
    fn endpoints(&self) -> Result<crate::endpoints::Endpoints, String> {
        Ok(crate::endpoints::Endpoints::default())
    }
}

/// How a vault walks its root: hidden entries skipped, and a link read as
/// what it points to.
const VAULT_WALK: crate::walk::Walk = crate::walk::Walk {
    links: crate::walk::Links::Follow,
    hidden: false,
};

/// Markdown files under a directory, excluding hidden entries.
pub struct Filesystem {
    root: PathBuf,
}

impl Filesystem {
    /// Select the directory to read.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
        }
    }
}

impl VaultSource for Filesystem {
    fn read(&self) -> io::Result<Snapshot> {
        // The sorted walk `io.fs` lists entries in, so the documents are
        // those of `walk(folder(root))`, in its order.
        let mut documents = Vec::new();
        crate::walk::walk::<io::Error>(&self.root, VAULT_WALK, &mut |listed| {
            if listed.stat.kind == crate::walk::Kind::File
                && let Some(format) = Format::of(&listed.relative)
            {
                documents.push(SourceDocument {
                    text: std::fs::read_to_string(self.root.join(&listed.relative)),
                    path: listed.relative,
                    format,
                });
            }
            Ok(true)
        })?;
        Ok(Snapshot {
            root: self.root.clone(),
            documents,
        })
    }

    fn extensions(&self) -> crate::extensions::Config {
        crate::extensions::Config::read(&self.root)
    }

    fn semantics(&self) -> crate::extensions::semantic::Semantics {
        crate::extensions::semantic::Semantics::read(&self.root)
    }

    fn formats(&self) -> Result<crate::extensions::format::Renderers, String> {
        crate::extensions::format::Renderers::read(&self.root)
    }

    fn ai(&self) -> crate::extensions::ai::transport::Settings {
        crate::extensions::ai::transport::Settings::read(&self.root)
    }

    fn endpoints(&self) -> Result<crate::endpoints::Endpoints, String> {
        crate::endpoints::Endpoints::read(&self.root)
    }
}
