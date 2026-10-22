//! Source I/O, independent of document parsing and knowledge assembly, and
//! the data sources a run context configures under `[sources.<id>]`. See
//! `doc/proposals/HQL-0012/README.md`.

use crate::document::Format;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    /// Renderer choices supplied by this source.
    ///
    /// # Errors
    /// Names a configured renderer this binary does not have.
    fn formats(&self) -> Result<crate::extensions::format::Renderers, String> {
        Ok(crate::extensions::format::Renderers::default())
    }
    /// The data sources this source's configuration declares.
    ///
    /// # Errors
    /// Names the first malformed `[sources.<id>]` entry.
    fn sources(&self) -> Result<Sources, String> {
        Ok(Sources::default())
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

    fn formats(&self) -> Result<crate::extensions::format::Renderers, String> {
        crate::extensions::format::Renderers::read(&self.root)
    }

    fn sources(&self) -> Result<Sources, String> {
        Sources::read(&self.root)
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

/// The kinds of configured source this binary reads, each with the keys its
/// `[sources.<id>]` table takes besides `kind` and `element`.
pub const KINDS: [(&str, &[&str]); 6] = [
    ("vault", &["root"]),
    ("csv", &["path"]),
    ("json", &["path"]),
    ("json_lines", &["path"]),
    ("yaml", &["path"]),
    ("sqlite", &["path", "table", "order"]),
];

/// The keys that hold a secret. A configuration names the environment
/// variable that holds one, `password_env`, and never the secret itself.
const SECRET_KEYS: [&str; 4] = ["password", "token", "secret", "api_key"];

/// What a configured source is read from, by kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Documents under a directory, read as cards: `kind = "vault"`.
    Vault {
        /// The directory, resolved against the configuring vault.
        root: PathBuf,
    },
    /// A file of documents in one format: `csv`, `json` (the elements of
    /// one array), `json_lines` or `yaml`.
    File {
        /// The file, resolved against the configuring vault.
        path: PathBuf,
        /// The format its bytes are read in, `Csv`.
        format: &'static str,
    },
    /// One table of an SQLite database, a row per element.
    Sqlite {
        /// The database file, resolved against the configuring vault.
        path: PathBuf,
        /// The table read.
        table: String,
        /// The columns its rows are ordered by; empty orders by `rowid`.
        order: Vec<String>,
    },
}

impl Kind {
    /// The kind's name, as `kind =` writes it.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Vault { .. } => "vault",
            Self::File { format, .. } => match *format {
                "Csv" => "csv",
                "Json" => "json",
                "JsonLines" => "json_lines",
                _ => "yaml",
            },
            Self::Sqlite { .. } => "sqlite",
        }
    }
}

/// One configured source: its id, what it is read from, and the element
/// type its configuration names.
#[derive(Debug, Clone)]
pub struct Configured {
    /// The id a program names it by.
    pub id: String,
    /// What it is read from.
    pub kind: Kind,
    /// The element type `element =` names, when it names one.
    pub element: Option<String>,
    /// A vault source, loaded once: every use in a run sees one snapshot.
    loaded: Arc<std::sync::OnceLock<Result<Arc<crate::vault::Vault>, String>>>,
}

impl Configured {
    /// A vault source's vault, loaded on first use and kept, so every use of
    /// one source within a run observes the same cards.
    ///
    /// # Errors
    /// Names the source when this is not a vault source or its directory
    /// cannot be read.
    pub fn vault(&self) -> Result<Arc<crate::vault::Vault>, String> {
        let Kind::Vault { root } = &self.kind else {
            return Err(format!("source `{}` is not a vault", self.id));
        };
        self.loaded
            .get_or_init(|| {
                crate::vault::load(root)
                    .map(Arc::new)
                    .map_err(|error| format!("source `{}` ({}): {error}", self.id, root.display()))
            })
            .clone()
    }
}

/// The bounds a run context sets on what one evaluation holds of a source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    /// The most elements of one source an evaluation may hold.
    pub max_elements: Option<usize>,
    /// The most bytes of one source an evaluation may read.
    pub max_bytes: Option<u64>,
}

/// The data sources a run context configures: `[sources.<id>]` in the
/// vault's `hql.toml`, and the bounds `[execution] max_elements` and
/// `max_bytes`.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    entries: BTreeMap<String, Configured>,
    /// What one evaluation may hold of a source.
    pub bounds: Bounds,
}

impl Sources {
    /// Read the sources a vault's `hql.toml` configures; none without one.
    ///
    /// # Errors
    /// Names the first entry that is malformed: an id off `[a-z][a-z0-9_]*`,
    /// the reserved id `vault`, a literal secret, an unknown kind, a missing
    /// or unknown key.
    pub fn read(root: &Path) -> Result<Self, String> {
        let Ok(text) = std::fs::read_to_string(root.join(crate::reporting::CONFIG_FILE)) else {
            return Ok(Self::default());
        };
        let parsed: toml::Value = toml::from_str(&text)
            .map_err(|error| format!("{}: {error}", crate::reporting::CONFIG_FILE))?;
        Self::parse(&parsed, root)
    }

    /// The sources a parsed configuration declares, paths resolved against
    /// `root`.
    ///
    /// # Errors
    /// As [`Sources::read`].
    pub fn parse(parsed: &toml::Value, root: &Path) -> Result<Self, String> {
        let mut sources = Self {
            bounds: bounds(parsed.get("execution"))?,
            ..Self::default()
        };
        let Some(section) = parsed.get("sources") else {
            return Ok(sources);
        };
        let section = section
            .as_table()
            .ok_or("`sources` holds one table per source, `[sources.<id>]`")?;
        for (id, entry) in section {
            let configured = configured(id, entry, root)?;
            sources.entries.insert(id.clone(), configured);
        }
        Ok(sources)
    }

    /// The source configured under an id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Configured> {
        self.entries.get(id)
    }

    /// The configured id nearest to `id` by edit distance, within 2.
    #[must_use]
    pub fn nearest(&self, id: &str) -> Option<&str> {
        self.entries
            .keys()
            .map(|known| (crate::builtins::distance(id, known), known.as_str()))
            .filter(|(distance, _)| *distance <= 2)
            .min()
            .map(|(_, known)| known)
    }
}

/// Whether text is a source id: `snake_case`, `[a-z][a-z0-9_]*`.
#[must_use]
pub fn is_id(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn bounds(execution: Option<&toml::Value>) -> Result<Bounds, String> {
    let bound = |key: &str| -> Result<Option<u64>, String> {
        match execution.and_then(|section| section.get(key)) {
            None => Ok(None),
            Some(value) => value
                .as_integer()
                .and_then(|n| u64::try_from(n).ok())
                .map(Some)
                .ok_or_else(|| format!("`[execution] {key}` is a count, not {value}")),
        }
    };
    Ok(Bounds {
        max_elements: bound("max_elements")?.map(|n| usize::try_from(n).unwrap_or(usize::MAX)),
        max_bytes: bound("max_bytes")?,
    })
}

fn configured(id: &str, entry: &toml::Value, root: &Path) -> Result<Configured, String> {
    if !is_id(id) {
        return Err(format!(
            "`[sources.{id}]`: a source id is snake_case, `[a-z][a-z0-9_]*`"
        ));
    }
    if id == crate::checker::VAULT {
        return Err(
            "`[sources.vault]` is not declared: `vault` is the vault the program runs against"
                .to_owned(),
        );
    }
    let table = entry
        .as_table()
        .ok_or_else(|| format!("`[sources.{id}]` is a table"))?;
    for key in SECRET_KEYS {
        if table.contains_key(key) {
            return Err(format!(
                "`[sources.{id}] {key}` holds a secret; name the environment variable that \
                 holds it, `{key}_env = \"VARIABLE\"`"
            ));
        }
    }
    let text = |key: &str| -> Result<Option<String>, String> {
        match table.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(|text| Some(text.to_owned()))
                .ok_or_else(|| format!("`[sources.{id}] {key}` is a string")),
        }
    };
    let kind = text("kind")?
        .ok_or_else(|| format!("`[sources.{id}]` names its `kind`: one of {}", kind_names()))?;
    let Some((kind, keys)) = KINDS.iter().find(|(name, _)| *name == kind) else {
        return Err(format!(
            "`[sources.{id}]` names the unknown kind `{kind}`; the kinds are {}",
            kind_names()
        ));
    };
    if let Some(key) = table.keys().find(|key| {
        !matches!(key.as_str(), "kind" | "element")
            && !keys.contains(&key.as_str())
            && !key.ends_with("_env")
    }) {
        return Err(format!(
            "`[sources.{id}]` of kind `{kind}` takes no `{key}`; it takes {}",
            keys.iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let required = |key: &str| -> Result<String, String> {
        text(key)?.ok_or_else(|| format!("`[sources.{id}]` of kind `{kind}` names its `{key}`"))
    };
    let kind = match *kind {
        "vault" => Kind::Vault {
            root: root.join(required("root")?),
        },
        "sqlite" => Kind::Sqlite {
            path: root.join(required("path")?),
            table: required("table")?,
            order: match table.get("order") {
                None => Vec::new(),
                Some(order) => order
                    .as_array()
                    .and_then(|columns| {
                        columns
                            .iter()
                            .map(|column| column.as_str().map(str::to_owned))
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| {
                        format!("`[sources.{id}] order` lists column names, `[\"id\"]`")
                    })?,
            },
        },
        file => Kind::File {
            path: root.join(required("path")?),
            format: match file {
                "csv" => "Csv",
                "json" => "Json",
                "json_lines" => "JsonLines",
                _ => "Yaml",
            },
        },
    };
    Ok(Configured {
        id: id.to_owned(),
        kind,
        element: text("element")?,
        loaded: Arc::default(),
    })
}

fn kind_names() -> String {
    KINDS
        .iter()
        .map(|(name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The rows of one SQLite table, in the order the configuration states, each
/// a `Data` map from column to value. The database is opened read-only.
///
/// # Errors
/// Names the database and what failed: opening it, or the query.
pub fn sqlite_rows(
    path: &Path,
    table: &str,
    order: &[String],
) -> Result<Vec<crate::data::Data>, String> {
    use crate::data::Data;
    use rusqlite::types::ValueRef;
    let located = |error: rusqlite::Error| format!("{}: {error}", path.display());
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(located)?;
    let quoted = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    let order = if order.is_empty() {
        "rowid".to_owned()
    } else {
        order
            .iter()
            .map(|column| quoted(column))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut statement = connection
        .prepare(&format!("SELECT * FROM {} ORDER BY {order}", quoted(table)))
        .map_err(located)?;
    let columns: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut rows = statement.query([]).map_err(located)?;
    let mut read = Vec::new();
    while let Some(row) = rows.next().map_err(located)? {
        let mut fields = BTreeMap::new();
        for (index, column) in columns.iter().enumerate() {
            let value = match row.get_ref(index).map_err(located)? {
                ValueRef::Null => Data::Empty,
                ValueRef::Integer(n) => Data::Int(n),
                ValueRef::Real(x) => Data::Float(x),
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
                    Data::Str(String::from_utf8_lossy(bytes).into_owned())
                }
            };
            fields.insert(column.clone(), value);
        }
        read.push(Data::Map(fields));
    }
    Ok(read)
}
