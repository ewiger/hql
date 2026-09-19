//! Source I/O, independent of document parsing and knowledge assembly.

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
}

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
        let mut paths = Vec::new();
        collect(&self.root, &mut paths)?;
        paths.sort();
        let documents = paths
            .into_iter()
            .filter_map(|path| {
                let format = Format::of(&path)?;
                let text = std::fs::read_to_string(&path);
                Some(SourceDocument {
                    path: path.strip_prefix(&self.root).unwrap_or(&path).to_owned(),
                    format,
                    text,
                })
            })
            .collect();
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
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            collect(&path, into)?;
        } else if Format::of(&path).is_some() {
            into.push(path);
        }
    }
    Ok(())
}
