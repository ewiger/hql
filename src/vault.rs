//! Loading a vault: documents on a filesystem, the cards built from them, and
//! the document graph their references write.
//!
//! An extension owns type knowledge and operations, never the data. This
//! module is the filesystem source; nothing above it knows what a file is.

use crate::data::Data;
use crate::document::{Card, Document, Format, Kind};
use crate::extensions::Config;
use crate::graph::{Edge, EdgeKind};
use crate::warnings::Warning;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs, io};

/// A namespace of documents, the cards built from them, and their edges.
#[derive(Debug, Clone)]
pub struct Vault {
    /// Where the vault was loaded from; also the index name a retrieval reports.
    pub root: PathBuf,
    /// Every card, in canonical name order.
    pub cards: Vec<Rc<Card>>,
    /// Lookup by name.
    pub by_name: BTreeMap<String, Rc<Card>>,
    /// Link and relation edges over the vault.
    pub edges: Vec<Edge>,
    /// What loading wanted to say without refusing to load.
    pub warnings: Vec<String>,
    /// What the vault's `hql.toml` says about extensions.
    pub extensions: Config,
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
    pub fn resolve(&self, name: &str) -> Option<Rc<Card>> {
        self.by_name.get(name).map(Rc::clone)
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
    let mut files = Vec::new();
    collect(root, &mut files)?;
    files.sort();

    // A stem is the name unless two documents share one, in which case both
    // take their path, because a namespace cannot hold one name twice.
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files {
        *seen.entry(stem(path)).or_default() += 1;
    }

    let mut warnings = Vec::new();
    let mut documents = Vec::new();
    for path in &files {
        let Some(format) = Format::of(path) else {
            continue;
        };
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                warnings.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        let stem = stem(path);
        let name = if seen.get(&stem).copied().unwrap_or_default() > 1 {
            relative_name(root, path)
        } else {
            stem
        };
        documents.push(Document::parse(&name, path, format, &source));
    }

    let names: BTreeSet<String> = documents.iter().map(|doc| doc.name.clone()).collect();
    let mut cards = Vec::new();
    for document in documents {
        cards.push(Rc::new(build_card(document, &names, &mut warnings)));
    }
    cards.sort_by(|left, right| left.name().cmp(right.name()));

    let by_name: BTreeMap<String, Rc<Card>> = cards
        .iter()
        .map(|card| (card.name().to_owned(), Rc::clone(card)))
        .collect();
    let edges = build_edges(&cards);

    Ok(Vault {
        root: root.to_owned(),
        cards,
        by_name,
        edges,
        warnings,
        extensions: Config::read(root),
    })
}

/// Assemble a card's knowledge layer and check the kind it declares.
///
/// An authored `knowledge.type: Relation` is a claim. When its endpoints do
/// not resolve the claim fails, and the card stays a concept card with the
/// failed claim retained rather than discarded.
fn build_card(document: Document, names: &BTreeSet<String>, warnings: &mut Vec<String>) -> Card {
    let mut metadata = match document.header.path("metadata") {
        Some(Data::Map(entries)) => Data::Map(entries.clone()),
        _ => Data::map(),
    };

    let declared = metadata
        .path("knowledge.type")
        .and_then(Data::as_str)
        .map(str::to_owned);
    let endpoints = || {
        let source = metadata
            .path("knowledge.relation.source")
            .and_then(Data::as_str)?;
        let target = metadata
            .path("knowledge.relation.target")
            .and_then(Data::as_str)?;
        Some((source.to_owned(), target.to_owned()))
    };

    let kind = if declared.as_deref() == Some("Relation") {
        match endpoints() {
            Some((source, target)) if names.contains(&source) && names.contains(&target) => {
                Kind::Relation
            }
            _ => {
                warnings.push(format!(
                    "{}: declares knowledge.type: Relation but its endpoints do not resolve; \
                     kept as a ConceptCard",
                    document.name
                ));
                metadata.insert_path(
                    "knowledge.unverified",
                    Data::Str("relation endpoints do not resolve".to_owned()),
                );
                Kind::Concept
            }
        }
    } else {
        Kind::Concept
    };

    // Extensions contribute under a key they own, so an authored entry and a
    // contributed one can never collide and no precedence rule is needed.
    metadata.insert_path("search.model", Data::Str(crate::search::MODEL.to_owned()));
    metadata.insert_path("search.indexed", Data::Bool(true));

    Card {
        document,
        metadata,
        kind,
    }
}

/// Collect link edges from references and relation edges from relation cards.
fn build_edges(cards: &[Rc<Card>]) -> Vec<Edge> {
    // A document edge is identified by its endpoints: writing the link twice
    // addresses it twice and merges the annotation.
    let mut links: BTreeMap<(String, String), Data> = BTreeMap::new();
    for card in cards {
        for reference in &card.document.references {
            let key = (card.name().to_owned(), reference.target.clone());
            let entry = links.entry(key).or_insert_with(Data::map);
            if let (Data::Map(into), Data::Map(from)) = (&mut *entry, &reference.data) {
                for (key, value) in from {
                    into.insert(key.clone(), value.clone());
                }
            }
        }
    }

    let mut edges: Vec<Edge> = links
        .into_iter()
        .map(|((source, target), data)| Edge {
            source,
            target,
            data,
            kind: EdgeKind::Link,
        })
        .collect();

    // A knowledge edge is identified by itself, so relation cards never merge.
    for card in cards {
        if card.kind == Kind::Relation
            && let Some((source, target)) = card.endpoints()
        {
            let mut data = Data::map();
            data.insert_path("card", Data::Str(card.name().to_owned()));
            edges.push(Edge {
                source,
                target,
                data,
                kind: EdgeKind::Relation,
            });
        }
    }
    edges
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let hidden = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'));
        if hidden {
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

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn relative_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
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
