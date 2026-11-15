//! How a vault's documents are read, independent of document parsing and
//! knowledge assembly. See `doc/proposals/HQL-0012/index.hmd`.

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
    /// Source identity, used for retrieval provenance: the vault root.
    pub root: PathBuf,
    /// Documents in this snapshot; assembly does not depend on their order.
    pub documents: Vec<SourceDocument>,
}

/// A source supplies text and configuration without assembling knowledge.
pub trait VaultSource: Send + Sync {
    /// Read a source snapshot, failing if the source itself is unreadable.
    fn read(&self) -> io::Result<Snapshot>;
    /// The directory the documents' paths are relative to, where it is not
    /// the snapshot's root: a vault's namespace root.
    fn namespace(&self) -> Option<PathBuf> {
        None
    }
    /// Extensions enabled by this source.
    fn extensions(&self) -> crate::extensions::Config {
        crate::extensions::Config::default()
    }
    /// Semantic index configuration supplied by this source.
    fn semantics(&self) -> crate::extensions::semantic::Semantics {
        crate::extensions::semantic::Semantics::default()
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

/// The cards of a vault on a filesystem, excluding hidden entries.
pub struct Filesystem {
    /// The vault root, holding `.hmd/`; or a plain directory read whole.
    root: PathBuf,
    /// The namespace root: where the cards start.
    cards: PathBuf,
}

impl Filesystem {
    /// Select the vault to read. Its cards are read from its namespace root,
    /// the directory `wiki` in its `.hmd/config.toml` names, as HyperMarkdown
    /// reads them; a directory that holds no `.hmd/` is read whole.
    ///
    /// # Errors
    /// The vault's `.hmd/config.toml` is malformed.
    pub fn new(root: &Path) -> io::Result<Self> {
        Ok(Self {
            cards: namespace_root(root)?,
            root: root.to_owned(),
        })
    }
}

/// Where a vault's cards start: `wiki` against the vault root, as
/// HyperMarkdown's own resolver reads it, or the directory itself where it
/// holds no `.hmd/`.
///
/// # Errors
/// The vault's `.hmd/config.toml` is malformed.
pub fn namespace_root(root: &Path) -> io::Result<PathBuf> {
    if !crate::vault::is_root(root) {
        return Ok(root.to_owned());
    }
    let config = hypermarkdown::Config::read(root)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(config.namespace_root(root))
}

impl VaultSource for Filesystem {
    fn read(&self) -> io::Result<Snapshot> {
        // The sorted walk `io.fs` lists entries in, so the documents are
        // those of `walk(folder(root))`, in its order.
        //
        // A folder below the root that is a vault root itself is another
        // vault, and its documents are not this one's: `discover` stops at the
        // nearest marker, so a card in there resolves against its own root.
        // The walk visits a folder before what it holds, so the nested roots
        // are known by the time their documents come up.
        let mut documents = Vec::new();
        let mut nested: Vec<PathBuf> = Vec::new();
        if !self.cards.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "the cards location {} does not exist; set `wiki` in {}",
                    self.cards.display(),
                    self.root.join(".hmd").join("config.toml").display()
                ),
            ));
        }
        crate::walk::walk::<io::Error>(&self.cards, VAULT_WALK, &mut |listed| {
            if nested
                .iter()
                .any(|vault| listed.relative.starts_with(vault))
            {
                return Ok(true);
            }
            if listed.stat.kind == crate::walk::Kind::Folder
                && crate::vault::is_root(&self.cards.join(&listed.relative))
            {
                nested.push(listed.relative);
                return Ok(true);
            }
            if listed.stat.kind == crate::walk::Kind::File
                && let Some(format) = Format::of(&listed.relative)
            {
                documents.push(SourceDocument {
                    text: std::fs::read_to_string(self.cards.join(&listed.relative)),
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

    fn namespace(&self) -> Option<PathBuf> {
        Some(self.cards.clone())
    }

    // A directory of documents configures its index, which is the vault's
    // own (HQL-0059); modules, renderers and endpoints are a project's, and
    // the project is taken apart from the documents.
    fn semantics(&self) -> crate::extensions::semantic::Semantics {
        crate::extensions::semantic::Semantics::read(&self.root)
    }
}
