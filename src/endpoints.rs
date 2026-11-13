//! The endpoints a run context configures under `[endpoint.<id>]` in
//! `hql.toml`: what each is, its schema boundary (HQL-0035), and the
//! bounds on what one evaluation holds of it. A program names one by its
//! id; `source` and `tap` read it, `target` and `sink` write it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The kinds of configured endpoint this binary reads, each with the keys its
/// `[endpoint.<id>]` table takes besides `kind` and `element`.
pub const KINDS: [(&str, &[&str]); 11] = [
    ("vault", &["root"]),
    ("csv", &["path"]),
    ("json", &["path"]),
    ("json_lines", &["path"]),
    ("yaml", &["path"]),
    ("sqlite", &["path", "table", "order"]),
    ("postgres", &["dsn_env", "table", "order"]),
    (
        "kafka",
        &["brokers", "brokers_env", "topic", "from", "idle_ms"],
    ),
    ("nats", &["url", "url_env", "subject", "idle_ms"]),
    (
        "nats_jetstream",
        &[
            "url", "url_env", "stream", "subject", "from", "idle_ms", "consumer",
        ],
    ),
    ("spark", &["url", "url_env", "token_env", "table", "order"]),
];

/// The keys that hold a secret. A configuration names the environment
/// variable that holds one, `password_env`, and never the secret itself.
const SECRET_KEYS: [&str; 5] = ["password", "token", "secret", "api_key", "dsn"];

/// What a configured endpoint is, by kind.
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
    /// One table of a PostgreSQL database, a row per element.
    Postgres {
        /// The environment variable that holds the connection string,
        /// `dsn_env`: a connection string may hold a password, so it is
        /// never written in the configuration.
        dsn_env: String,
        /// The table read, schema-qualified where it holds a dot.
        table: String,
        /// The columns its rows are ordered by; empty reads them as the
        /// table is scanned.
        order: Vec<String>,
    },
    /// One Kafka topic, a JSON document per message.
    Kafka {
        /// The brokers a connection starts from.
        brokers: Brokers,
        /// The topic read and written.
        topic: String,
        /// Where a read starts in each partition, `from =`.
        start: Start,
        /// How long a read waits for a message before its stream ends,
        /// `idle_ms =`; without it the stream never ends.
        idle: Option<std::time::Duration>,
    },
    /// One core NATS subject, a JSON document per message: what is
    /// published while a read is subscribed, and nothing kept.
    Nats {
        /// The server a connection starts from.
        server: Address,
        /// The subject read, which may hold wildcards, and published to.
        subject: String,
        /// How long a read waits for a message before its stream ends,
        /// `idle_ms =`; without it the stream never ends.
        idle: Option<std::time::Duration>,
    },
    /// One subject of a NATS JetStream stream, a JSON document per message.
    NatsJetStream {
        /// The server a connection starts from.
        server: Address,
        /// The stream, the subject within it, and how it is read.
        held: Subject,
    },
    /// One table behind a Spark SQL endpoint, a Databricks SQL warehouse or
    /// a Spark Thrift Server, a row per element.
    Spark {
        /// The endpoint's HTTP address.
        server: Address,
        /// The environment variable that holds an access token, `token_env`,
        /// where the endpoint asks for one.
        token_env: Option<String>,
        /// The table read, qualified by catalog and schema where it holds
        /// dots.
        table: String,
        /// The columns its rows are ordered by; empty reads them as the
        /// table is scanned.
        order: Vec<String>,
    },
}

/// Where reading a log starts: in each partition of a topic, or in a
/// stream.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Start {
    /// The earliest message the log still keeps.
    #[default]
    Earliest,
    /// The first message written after the read begins.
    Latest,
}

impl Start {
    /// The word `from =` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Earliest => "earliest",
            Self::Latest => "latest",
        }
    }
}

/// One subject of a NATS JetStream stream, and how a read of it proceeds:
/// what the `nats_jetstream` kind configures and the `nats_jetstream`
/// constructor of `io.msg.nats_jetstream` takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The stream that holds the messages read.
    pub stream: String,
    /// The subject read within the stream, and published to.
    pub subject: String,
    /// Where a read starts in the stream, `from =`.
    pub start: Start,
    /// How long a read waits for a message before its stream ends,
    /// `idle_ms =`; without it the stream never ends.
    pub idle: Option<std::time::Duration>,
    /// The durable consumer that keeps the read's position on the server,
    /// `consumer =`; without it a read keeps none.
    pub consumer: Option<String>,
}

impl Subject {
    /// A subject as its keys state it, each checked.
    ///
    /// # Errors
    /// Names the key whose value is not one it takes.
    pub fn new(
        stream: String,
        subject: String,
        from: Option<&str>,
        idle_ms: Option<i64>,
        consumer: Option<String>,
    ) -> Result<Self, String> {
        let start = match from {
            None | Some("earliest") => Start::Earliest,
            Some("latest") => Start::Latest,
            Some(other) => {
                return Err(format!("`from` is `earliest` or `latest`, not `{other}`"));
            }
        };
        let idle = match idle_ms {
            None => None,
            Some(ms) => Some(std::time::Duration::from_millis(
                u64::try_from(ms)
                    .ok()
                    .filter(|ms| *ms > 0)
                    .ok_or("`idle_ms` is a count of milliseconds, above 0")?,
            )),
        };
        if let Some(name) = &consumer
            && (name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        {
            return Err(format!(
                "`consumer` is a name of letters, digits, `_` and `-`, not `{name}`"
            ));
        }
        Ok(Self {
            stream,
            subject,
            start,
            idle,
            consumer,
        })
    }
}

/// The address of a NATS server as `url` or `url_env` names it, once.
///
/// # Errors
/// Says so when neither or both are written, or `url` holds credentials.
pub fn nats_address(
    url: Option<String>,
    url_env: Option<String>,
    kind: &str,
) -> Result<Address, String> {
    match (url, url_env) {
        (Some(url), None) if url.contains('@') => Err(
            "`url` holds credentials; name the environment variable that holds the address, \
             `url_env = \"VARIABLE\"`"
                .to_owned(),
        ),
        (Some(url), None) => Ok(Address::Written(url)),
        (None, Some(variable)) => Ok(Address::Env(variable)),
        _ => Err(format!(
            "a `{kind}` endpoint names its address once, with `url = \"nats://host:4222\"` or \
             `url_env = \"VARIABLE\"`"
        )),
    }
}

/// Where a NATS or Spark endpoint's address is named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    /// In the configuration, `url = "nats://host:4222"`.
    Written(String),
    /// In the environment variable `url_env` names.
    Env(String),
}

impl Address {
    /// The address, read from the environment where the configuration names
    /// a variable.
    ///
    /// # Errors
    /// Names the variable when it is not set.
    pub fn resolve(&self) -> Result<String, String> {
        match self {
            Self::Written(url) => Ok(url.clone()),
            Self::Env(variable) => secret(variable, "url_env"),
        }
    }
}

/// Where a Kafka endpoint's brokers are named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Brokers {
    /// In the configuration, `brokers = "host:9092"`.
    Listed(Vec<String>),
    /// In the environment variable `brokers_env` names, as a list separated
    /// by commas.
    Env(String),
}

impl Brokers {
    /// The brokers, read from the environment where the configuration names
    /// a variable.
    ///
    /// # Errors
    /// Names the variable when it is not set or names no broker.
    pub fn resolve(&self) -> Result<Vec<String>, String> {
        match self {
            Self::Listed(brokers) => Ok(brokers.clone()),
            Self::Env(variable) => {
                let listed = std::env::var(variable).map_err(|_| {
                    format!("the environment variable `{variable}`, which `brokers_env` names, is not set")
                })?;
                let brokers = split_brokers(&listed);
                if brokers.is_empty() {
                    return Err(format!(
                        "the environment variable `{variable}` names no broker"
                    ));
                }
                Ok(brokers)
            }
        }
    }
}

fn split_brokers(listed: &str) -> Vec<String> {
    listed
        .split(',')
        .map(str::trim)
        .filter(|broker| !broker.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The value of the environment variable a `*_env` key names.
///
/// # Errors
/// Names the variable and the key when it is not set.
pub fn secret(variable: &str, key: &str) -> Result<String, String> {
    std::env::var(variable).map_err(|_| {
        format!("the environment variable `{variable}`, which `{key}` names, is not set")
    })
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
            Self::Postgres { .. } => "postgres",
            Self::Kafka { .. } => "kafka",
            Self::Nats { .. } => "nats",
            Self::NatsJetStream { .. } => "nats_jetstream",
            Self::Spark { .. } => "spark",
        }
    }

    /// Whether the kind is a log read as it arrives, by `tap`, rather than
    /// a finite one read by `source`.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(
            self,
            Self::Kafka { .. } | Self::Nats { .. } | Self::NatsJetStream { .. }
        )
    }
}

/// How a write to a configured endpoint treats what it already holds
/// (HQL-0035).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WriteMode {
    /// Written only where nothing is held yet.
    Create,
    /// Added after what is held, created where nothing is.
    Append,
    /// What is held replaced whole.
    #[default]
    Replace,
}

impl WriteMode {
    /// The word `mode =` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Append => "append",
            Self::Replace => "replace",
        }
    }
}

/// What a read does with an external field no HQL field reads (HQL-0035).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Strictness {
    /// It is a boundary error.
    Strict,
    /// It passes to the HQL type, whose openness decides.
    #[default]
    Compatible,
    /// It is not read.
    Open,
    /// It is not read, and is reported as a warning.
    Warn,
}

impl Strictness {
    /// The word `strictness =` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Compatible => "compatible",
            Self::Open => "open",
            Self::Warn => "warn",
        }
    }
}

/// Where an HQL field is held outside: a name, or a path into nested data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum External {
    /// A CSV header, a JSON key or a column, `name = "user_id"`.
    Name(String),
    /// A field below the top level, `path = "profile.display_name"`.
    Path(Vec<String>),
}

/// An external representation an optional field reads as absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absence {
    /// The record holds no such field.
    Missing,
    /// The field holds empty text, a CSV cell with nothing in it.
    Empty,
    /// The field holds JSON `null` or SQL `NULL`.
    Null,
}

impl Absence {
    /// The word `absent = [...]` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Empty => "empty",
            Self::Null => "null",
        }
    }
}

/// What a configuration says of one field's external side (HQL-0035): a
/// property it does not write is `None`, left to the format's default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldMapping {
    /// The external field, `name` or `path`.
    pub external: Option<External>,
    /// The external type, in the format's vocabulary.
    pub external_type: Option<String>,
    /// The external field's place in the record, from 1.
    pub position: Option<usize>,
    /// Whether the field is read.
    pub readable: Option<bool>,
    /// Whether the field is written.
    pub writable: Option<bool>,
    /// The external representations an optional field reads as absent.
    pub absent: Option<Vec<Absence>>,
    /// Whether a representation that may not keep the value is permitted.
    pub lossy: Option<bool>,
    /// The keys the kind's extension owns, under the kind's name.
    pub owned: BTreeMap<String, toml::Value>,
}

/// The core keys of a field's table (HQL-0035).
const FIELD_KEYS: [&str; 8] = [
    "name", "path", "type", "position", "read", "write", "absent", "lossy",
];

/// The field keys each kind's extension owns, under its own name: none yet.
fn owned_field_keys(_kind: &str) -> &'static [&'static str] {
    &[]
}

/// One configured endpoint: its id, what it is, and the element type its
/// configuration names.
#[derive(Debug, Clone)]
pub struct Configured {
    /// The id a program names it by.
    pub id: String,
    /// What it is.
    pub kind: Kind,
    /// The element type `element =` names, when it names one.
    pub element: Option<String>,
    /// The boundary it configures, field by field, `[endpoint.<id>.fields]`.
    pub fields: BTreeMap<String, FieldMapping>,
    /// How a write to it treats what it holds, `mode =`.
    pub mode: WriteMode,
    /// What a read does with an unmapped external field, `strictness =`.
    pub strictness: Strictness,
    /// A vault endpoint, loaded once: every use in a run sees one snapshot.
    loaded: Arc<std::sync::OnceLock<Result<Arc<crate::vault::Vault>, String>>>,
}

impl Configured {
    /// A transport constructed in a program, as a configured endpoint with
    /// no boundary, `mode` or `strictness` of its own: `id` is how
    /// diagnostics name it.
    #[must_use]
    pub fn constructed(id: String, kind: Kind) -> Self {
        Self {
            id,
            kind,
            element: None,
            fields: BTreeMap::new(),
            mode: WriteMode::default(),
            strictness: Strictness::default(),
            loaded: Arc::default(),
        }
    }

    /// A vault endpoint's vault, loaded on first use and kept, so every use
    /// of one endpoint within a run observes the same cards.
    ///
    /// # Errors
    /// Names the endpoint when this is not a vault endpoint or its directory
    /// cannot be read.
    pub fn vault(&self) -> Result<Arc<crate::vault::Vault>, String> {
        let Kind::Vault { root } = &self.kind else {
            return Err(format!("endpoint `{}` is not a vault", self.id));
        };
        self.loaded
            .get_or_init(|| {
                crate::vault::load(root).map(Arc::new).map_err(|error| {
                    format!("endpoint `{}` ({}): {error}", self.id, root.display())
                })
            })
            .clone()
    }
}

/// The bounds a run context sets on what one evaluation holds of an endpoint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    /// The most elements of one endpoint an evaluation may hold.
    pub max_elements: Option<usize>,
    /// The most bytes of one endpoint an evaluation may read.
    pub max_bytes: Option<u64>,
}

/// The endpoints a run context configures: `[endpoint.<id>]` in the
/// vault's `hql.toml`, and the bounds `[execution] max_elements` and
/// `max_bytes`.
#[derive(Debug, Clone, Default)]
pub struct Endpoints {
    entries: BTreeMap<String, Configured>,
    /// What one evaluation may hold of an endpoint.
    pub bounds: Bounds,
}

impl Endpoints {
    /// Read the endpoints a vault's `hql.toml` configures; none without one.
    ///
    /// # Errors
    /// Names the first entry that is malformed: an id off `[a-z][a-z0-9_]*`,
    /// the reserved id `vault`, a `[sources]` table, a literal secret, an
    /// unknown kind, a missing or unknown key.
    pub fn read(root: &Path) -> Result<Self, String> {
        // Only a missing file means no endpoints; one that cannot be read is
        // an error, never an empty configuration.
        let text = match std::fs::read_to_string(root.join(crate::reporting::CONFIG_FILE)) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("{}: {error}", crate::reporting::CONFIG_FILE)),
        };
        let parsed: toml::Value = toml::from_str(&text)
            .map_err(|error| format!("{}: {error}", crate::reporting::CONFIG_FILE))?;
        Self::parse(&parsed, root)
    }

    /// The endpoints a parsed configuration declares, paths resolved against
    /// `root`.
    ///
    /// # Errors
    /// As [`Endpoints::read`].
    pub fn parse(parsed: &toml::Value, root: &Path) -> Result<Self, String> {
        let mut endpoints = Self {
            bounds: bounds(parsed.get("execution"))?,
            ..Self::default()
        };
        if parsed.get("sources").is_some() {
            return Err(
                "`[sources]` is not read; an endpoint is declared under `[endpoint.<id>]`"
                    .to_owned(),
            );
        }
        let Some(section) = parsed.get("endpoint") else {
            return Ok(endpoints);
        };
        let section = section
            .as_table()
            .ok_or("`endpoint` holds one table per endpoint, `[endpoint.<id>]`")?;
        for (id, entry) in section {
            let configured = configured(id, entry, root)?;
            endpoints.entries.insert(id.clone(), configured);
        }
        Ok(endpoints)
    }

    /// The endpoint configured under an id.
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

/// Whether text is an endpoint id: `snake_case`, `[a-z][a-z0-9_]*`.
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
            "`[endpoint.{id}]`: an endpoint id is snake_case, `[a-z][a-z0-9_]*`"
        ));
    }
    if id == crate::checker::VAULT {
        return Err(
            "`[endpoint.vault]` is not declared: `vault` is the vault the program runs against"
                .to_owned(),
        );
    }
    let table = entry
        .as_table()
        .ok_or_else(|| format!("`[endpoint.{id}]` is a table"))?;
    for key in SECRET_KEYS {
        if table.contains_key(key) {
            return Err(format!(
                "`[endpoint.{id}] {key}` holds a secret; name the environment variable that \
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
                .ok_or_else(|| format!("`[endpoint.{id}] {key}` is a string")),
        }
    };
    let kind = text("kind")?.ok_or_else(|| {
        format!(
            "`[endpoint.{id}]` names its `kind`: one of {}",
            kind_names()
        )
    })?;
    let Some((kind, keys)) = KINDS.iter().find(|(name, _)| *name == kind) else {
        return Err(format!(
            "`[endpoint.{id}]` names the unknown kind `{kind}`; the kinds are {}",
            kind_names()
        ));
    };
    if let Some(key) = table.keys().find(|key| {
        !matches!(key.as_str(), "kind" | "element")
            && !(matches!(key.as_str(), "fields" | "mode" | "strictness") && *kind != "vault")
            && !keys.contains(&key.as_str())
            && !(key.ends_with("_env")
                && !matches!(
                    *kind,
                    "postgres" | "kafka" | "nats" | "nats_jetstream" | "spark"
                ))
    }) {
        return Err(format!(
            "`[endpoint.{id}]` of kind `{kind}` takes no `{key}`; it takes {}",
            keys.iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let required = |key: &str| -> Result<String, String> {
        text(key)?.ok_or_else(|| format!("`[endpoint.{id}]` of kind `{kind}` names its `{key}`"))
    };
    let columns = |key: &str| -> Result<Vec<String>, String> {
        match table.get(key) {
            None => Ok(Vec::new()),
            Some(order) => order
                .as_array()
                .and_then(|columns| {
                    columns
                        .iter()
                        .map(|column| column.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| format!("`[endpoint.{id}] {key}` lists column names, `[\"id\"]`")),
        }
    };
    let start = || -> Result<Start, String> {
        match text("from")?.as_deref() {
            None | Some("earliest") => Ok(Start::Earliest),
            Some("latest") => Ok(Start::Latest),
            Some(other) => Err(format!(
                "`[endpoint.{id}] from` is `earliest` or `latest`, not `{other}`"
            )),
        }
    };
    let idle = || -> Result<Option<std::time::Duration>, String> {
        match table.get("idle_ms") {
            None => Ok(None),
            Some(value) => value
                .as_integer()
                .and_then(|ms| u64::try_from(ms).ok())
                .filter(|ms| *ms > 0)
                .map(|ms| Some(std::time::Duration::from_millis(ms)))
                .ok_or_else(|| {
                    format!("`[endpoint.{id}] idle_ms` is a count of milliseconds, above 0")
                }),
        }
    };
    // An address is named once. One written in the configuration holds no
    // credentials: those belong in the environment, with the address.
    let address = |http: bool| -> Result<Address, String> {
        let example = if http {
            "https://host/sql/1.0/warehouses/id"
        } else {
            "nats://host:4222"
        };
        match (text("url")?, text("url_env")?) {
            (Some(url), None) if url.contains('@') => Err(format!(
                "`[endpoint.{id}] url` holds credentials; name the environment variable that \
                 holds the address, `url_env = \"VARIABLE\"`"
            )),
            (Some(url), None) => Ok(Address::Written(url)),
            (None, Some(variable)) => Ok(Address::Env(variable)),
            _ => Err(format!(
                "`[endpoint.{id}]` of kind `{kind}` names its address once, with \
                 `url = \"{example}\"` or `url_env = \"VARIABLE\"`"
            )),
        }
    };
    let kind = match *kind {
        "postgres" => Kind::Postgres {
            dsn_env: text("dsn_env")?.ok_or_else(|| {
                format!(
                    "`[endpoint.{id}]` of kind `postgres` names the environment variable that \
                     holds its connection string, `dsn_env = \"VARIABLE\"`"
                )
            })?,
            table: required("table")?,
            order: columns("order")?,
        },
        "kafka" => Kind::Kafka {
            brokers: match (text("brokers")?, text("brokers_env")?) {
                (Some(listed), None) => {
                    let brokers = split_brokers(&listed);
                    if brokers.is_empty() {
                        return Err(format!("`[endpoint.{id}] brokers` names no broker"));
                    }
                    Brokers::Listed(brokers)
                }
                (None, Some(variable)) => Brokers::Env(variable),
                _ => {
                    return Err(format!(
                        "`[endpoint.{id}]` of kind `kafka` names its brokers once, with \
                         `brokers = \"host:9092\"` or `brokers_env = \"VARIABLE\"`"
                    ));
                }
            },
            topic: required("topic")?,
            start: start()?,
            idle: idle()?,
        },
        "nats" => Kind::Nats {
            server: nats_address(text("url")?, text("url_env")?, "nats")
                .map_err(|message| format!("`[endpoint.{id}]`: {message}"))?,
            subject: required("subject")?,
            idle: idle()?,
        },
        "nats_jetstream" => Kind::NatsJetStream {
            server: nats_address(text("url")?, text("url_env")?, "nats_jetstream")
                .map_err(|message| format!("`[endpoint.{id}]`: {message}"))?,
            held: Subject::new(
                required("stream")?,
                required("subject")?,
                text("from")?.as_deref(),
                // A value that is no integer is no count either.
                table
                    .get("idle_ms")
                    .map(|value| value.as_integer().unwrap_or(0)),
                text("consumer")?,
            )
            .map_err(|message| format!("`[endpoint.{id}]`: {message}"))?,
        },
        "spark" => Kind::Spark {
            server: address(true)?,
            token_env: text("token_env")?,
            table: required("table")?,
            order: columns("order")?,
        },
        "vault" => Kind::Vault {
            root: root.join(required("root")?),
        },
        "sqlite" => Kind::Sqlite {
            path: root.join(required("path")?),
            table: required("table")?,
            order: columns("order")?,
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
    let fields = match table.get("fields") {
        None => BTreeMap::new(),
        Some(fields) => boundary(id, kind.name(), fields)?,
    };
    let mode = match text("mode")?.as_deref() {
        None | Some("replace") => WriteMode::Replace,
        Some("append") => WriteMode::Append,
        Some("create") => WriteMode::Create,
        Some(other) => {
            return Err(format!(
                "`[endpoint.{id}] mode` is `create`, `append` or `replace`, not `{other}`"
            ));
        }
    };
    let strictness = match text("strictness")?.as_deref() {
        None | Some("compatible") => Strictness::Compatible,
        Some("strict") => Strictness::Strict,
        Some("open") => Strictness::Open,
        Some("warn") => Strictness::Warn,
        Some(other) => {
            return Err(format!(
                "`[endpoint.{id}] strictness` is `strict`, `compatible`, `open` or `warn`, not \
                 `{other}`"
            ));
        }
    };
    Ok(Configured {
        id: id.to_owned(),
        kind,
        element: text("element")?,
        fields,
        mode,
        strictness,
        loaded: Arc::default(),
    })
}

/// The boundary `[endpoint.<id>.fields]` configures, one mapping per field.
fn boundary(
    id: &str,
    kind: &str,
    fields: &toml::Value,
) -> Result<BTreeMap<String, FieldMapping>, String> {
    let fields = fields
        .as_table()
        .ok_or_else(|| format!("`[endpoint.{id}.fields]` holds one table per field"))?;
    fields
        .iter()
        .map(|(field, entry)| Ok((field.clone(), field_mapping(id, kind, field, entry)?)))
        .collect()
}

fn field_mapping(
    id: &str,
    kind: &str,
    field: &str,
    entry: &toml::Value,
) -> Result<FieldMapping, String> {
    let at = format!("[endpoint.{id}.fields.{field}]");
    let table = entry
        .as_table()
        .ok_or_else(|| format!("`{at}` is a table of the field's external properties"))?;
    if table.contains_key("name") && table.contains_key("path") {
        return Err(format!(
            "`{at}` names its external field once, with `name` or with `path`"
        ));
    }
    let mut mapping = FieldMapping::default();
    for (key, value) in table {
        let text = || {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("`{at} {key}` is a string"))
        };
        let truth = || {
            value
                .as_bool()
                .ok_or_else(|| format!("`{at} {key}` is `true` or `false`"))
        };
        match key.as_str() {
            "name" => mapping.external = Some(External::Name(text()?)),
            "path" => {
                if !matches!(
                    kind,
                    "json" | "json_lines" | "yaml" | "kafka" | "nats" | "nats_jetstream"
                ) {
                    return Err(format!(
                        "`{at} path` names a nested field, and a `{kind}` record does not \
                         nest; name its field with `name`"
                    ));
                }
                let written = text()?;
                let segments: Vec<String> = written.split('.').map(str::to_owned).collect();
                if segments.iter().any(String::is_empty) {
                    return Err(format!(
                        "`{at} path` is dotted field names, `profile.display_name`, not \
                         `{written}`"
                    ));
                }
                mapping.external = Some(External::Path(segments));
            }
            "type" => mapping.external_type = Some(text()?),
            "position" => {
                mapping.position = Some(
                    value
                        .as_integer()
                        .filter(|n| *n >= 1)
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or_else(|| {
                            format!("`{at} position` is the field's place, counted from 1")
                        })?,
                );
            }
            "read" => mapping.readable = Some(truth()?),
            "write" => mapping.writable = Some(truth()?),
            "lossy" => mapping.lossy = Some(truth()?),
            "absent" => {
                let absent = value
                    .as_array()
                    .and_then(|entries| {
                        entries
                            .iter()
                            .map(|entry| match entry.as_str()? {
                                "missing" => Some(Absence::Missing),
                                "empty" => Some(Absence::Empty),
                                "null" => Some(Absence::Null),
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| {
                        format!(
                            "`{at} absent` lists what reads as absent, from `\"missing\"`, \
                             `\"empty\"` and `\"null\"`"
                        )
                    })?;
                mapping.absent = Some(absent);
            }
            owner if owner == kind => {
                let owned = value
                    .as_table()
                    .ok_or_else(|| format!("`{at} {kind}` is a table of `{kind}`'s own keys"))?;
                let known = owned_field_keys(kind);
                if let Some(key) = owned.keys().find(|key| !known.contains(&key.as_str())) {
                    return Err(format!("`{at}`: `{kind}` owns no field key `{key}`"));
                }
                mapping.owned.extend(
                    owned
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
            }
            other if KINDS.iter().any(|(name, _)| *name == other) => {
                return Err(format!(
                    "`{at} {other}` holds `{other}`'s keys, and this endpoint is `{kind}`"
                ));
            }
            other => {
                return Err(format!(
                    "`{at}` takes no `{other}`; a field takes {}, and `{kind}`'s own keys \
                     under `{kind}`",
                    FIELD_KEYS
                        .iter()
                        .map(|key| format!("`{key}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
    Ok(mapping)
}

fn kind_names() -> String {
    KINDS
        .iter()
        .map(|(name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The columns of one SQLite table, in the table's order. The database is
/// opened read-only.
///
/// # Errors
/// Names the database and what failed: opening it, or the query.
pub fn sqlite_columns(path: &Path, table: &str) -> Result<Vec<String>, String> {
    on_sqlite(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY, |connection| {
        sqlite_columns_on(connection, path, table)
    })
}

/// Run `f` on a connection to the database file at `path`: the one an open
/// transaction scope holds (HQL-0051), or one opened with `flags` for this
/// call alone.
fn on_sqlite<T>(
    path: &Path,
    flags: rusqlite::OpenFlags,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let mut f = Some(f);
    if let Some(result) = crate::transaction::with_sqlite(path, |connection| {
        f.take().map(|f| f(connection))
    })
    .flatten()
    {
        return result;
    }
    let connection = rusqlite::Connection::open_with_flags(path, flags)
        .map_err(|error| crate::transaction::sqlite_message(path, &error))?;
    f.take().map_or_else(
        || Err("a connection was used twice".to_owned()),
        |f| f(&connection),
    )
}

fn sqlite_columns_on(
    connection: &rusqlite::Connection,
    path: &Path,
    table: &str,
) -> Result<Vec<String>, String> {
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    let mut statement = connection
        .prepare(&format!(
            "SELECT * FROM {} LIMIT 0",
            format_args!("\"{}\"", table.replace('"', "\"\""))
        ))
        .map_err(located)?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let _ = statement.query([]).map_err(located)?;
    Ok(columns)
}

/// The type each column of one SQLite table or view is declared with, by
/// column name; a column declared without one maps to the empty string.
///
/// # Errors
/// Names the database and what failed: opening it, or the query.
pub fn sqlite_column_types(path: &Path, table: &str) -> Result<BTreeMap<String, String>, String> {
    on_sqlite(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY, |connection| {
        sqlite_column_types_on(connection, path, table)
    })
}

fn sqlite_column_types_on(
    connection: &rusqlite::Connection,
    path: &Path,
    table: &str,
) -> Result<BTreeMap<String, String>, String> {
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    let mut statement = connection
        .prepare("SELECT name, type FROM pragma_table_info(?1)")
        .map_err(located)?;
    let rows = statement
        .query_map([table], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(located)?;
    rows.collect::<Result<_, _>>().map_err(located)
}

/// The rows of one SQLite table, in the order the configuration states, each
/// a `Data` map from column to value: every column, or only `columns` where
/// a boundary names the ones read. The database is opened read-only.
///
/// # Errors
/// Names the database and what failed: opening it, or the query.
pub fn sqlite_rows(
    path: &Path,
    table: &str,
    order: &[String],
    columns: Option<&[String]>,
) -> Result<Vec<crate::data::Data>, String> {
    on_sqlite(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY, |connection| {
        sqlite_rows_on(connection, path, table, order, columns)
    })
}

fn sqlite_rows_on(
    connection: &rusqlite::Connection,
    path: &Path,
    table: &str,
    order: &[String],
    columns: Option<&[String]>,
) -> Result<Vec<crate::data::Data>, String> {
    use crate::data::Data;
    use rusqlite::types::ValueRef;
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    let quoted = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    // Unordered, a table reads in `rowid` order; a view or a `WITHOUT ROWID`
    // table has none, and reads in the order SQLite scans it.
    let has_rowid = connection
        .prepare(&format!("SELECT rowid FROM {} LIMIT 0", quoted(table)))
        .is_ok();
    let order = if !order.is_empty() {
        let columns = order
            .iter()
            .map(|column| quoted(column))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" ORDER BY {columns}")
    } else if has_rowid {
        " ORDER BY rowid".to_owned()
    } else {
        String::new()
    };
    let selected = columns.filter(|columns| !columns.is_empty()).map_or_else(
        || "*".to_owned(),
        // Each under the name the boundary reads it by: SQLite matches a
        // column name without regard to case, and gives back its own.
        |columns| {
            columns
                .iter()
                .map(|column| format!("{} AS {}", quoted(column), quoted(column)))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let mut statement = connection
        .prepare(&format!("SELECT {selected} FROM {}{order}", quoted(table)))
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
                    Data::Str(String::from_utf8(bytes.to_vec()).map_err(|_| {
                        format!(
                            "{}: column `{column}` holds bytes that are not UTF-8 text",
                            path.display()
                        )
                    })?)
                }
            };
            fields.insert(column.clone(), value);
        }
        read.push(Data::Map(fields));
    }
    Ok(read)
}

/// Run one statement that changes rows of an SQLite database, `UPDATE` or
/// `DELETE`, with its parameters bound, and give the number of rows it
/// changed (HQL-0051). The database file must exist: a mutation changes
/// rows that exist.
///
/// # Errors
/// Names the database and what failed: opening it, or the statement.
pub(crate) fn sqlite_execute(
    path: &Path,
    statement: &str,
    params: &[crate::mutation::Param],
) -> Result<usize, String> {
    use crate::mutation::Param;
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    let values: Vec<rusqlite::types::Value> = params
        .iter()
        .map(|param| match param {
            Param::Int(n) => rusqlite::types::Value::Integer(*n),
            Param::Float(x) => rusqlite::types::Value::Real(*x),
            Param::Bool(b) => rusqlite::types::Value::Integer(i64::from(*b)),
            Param::Text(text) | Param::Numeric(text) => {
                rusqlite::types::Value::Text(text.clone())
            }
        })
        .collect();
    on_sqlite(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE, |connection| {
        connection
            .execute(statement, rusqlite::params_from_iter(values.iter()))
            .map_err(located)
    })
}

/// Write rows to one SQLite table as `mode` says, the table's columns each
/// a name and the type it is declared with, the rows in one transaction.
///
/// # Errors
/// Names the database and what failed: opening it, a table `create` finds
/// already there, or a statement.
pub fn sqlite_write(
    path: &Path,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
    rows: &[Vec<rusqlite::types::Value>],
) -> Result<(), String> {
    if columns.is_empty() {
        return Err(format!(
            "{}: no field of the record is written to table `{table}`; a field with \
             `write = false` is not a column",
            path.display()
        ));
    }
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    // Inside a transaction scope the rows join the attempt (HQL-0051);
    // outside one they are their own transaction.
    let mut inside = Some(|connection: &rusqlite::Connection| {
        sqlite_write_on(connection, path, table, mode, columns, rows)
    });
    if let Some(result) =
        crate::transaction::with_sqlite(path, |connection| inside.take().map(|f| f(connection)))
            .flatten()
    {
        return result;
    }
    let connection = rusqlite::Connection::open(path).map_err(located)?;
    let transaction = connection.unchecked_transaction().map_err(located)?;
    sqlite_write_on(&transaction, path, table, mode, columns, rows)?;
    transaction.commit().map_err(located)
}

fn sqlite_write_on(
    transaction: &rusqlite::Connection,
    path: &Path,
    table: &str,
    mode: WriteMode,
    columns: &[(String, String)],
    rows: &[Vec<rusqlite::types::Value>],
) -> Result<(), String> {
    let located = |error: rusqlite::Error| crate::transaction::sqlite_message(path, &error);
    let quoted = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    let held: i64 = transaction
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(located)?;
    match mode {
        WriteMode::Create if held > 0 => {
            return Err(format!(
                "{}: the table `{table}` exists, and `mode = \"create\"` writes only a new one",
                path.display()
            ));
        }
        WriteMode::Replace => {
            transaction
                .execute(&format!("DROP TABLE IF EXISTS {}", quoted(table)), [])
                .map_err(located)?;
        }
        WriteMode::Create | WriteMode::Append => {}
    }
    let declared: Vec<String> = columns
        .iter()
        .map(|(name, ty)| format!("{} {ty}", quoted(name)))
        .collect();
    transaction
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                quoted(table),
                declared.join(", ")
            ),
            [],
        )
        .map_err(located)?;
    {
        let names: Vec<String> = columns.iter().map(|(name, _)| quoted(name)).collect();
        let slots: Vec<String> = (1..=columns.len()).map(|n| format!("?{n}")).collect();
        let mut insert = transaction
            .prepare(&format!(
                "INSERT INTO {} ({}) VALUES ({})",
                quoted(table),
                names.join(", "),
                slots.join(", ")
            ))
            .map_err(located)?;
        for row in rows {
            insert
                .execute(rusqlite::params_from_iter(row.iter()))
                .map_err(located)?;
        }
    }
    Ok(())
}
