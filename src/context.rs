//! The execution context of an invocation (HQL-0025, HQL-0059): the HQL
//! project it runs in, if any, the HyperMarkdown vault it reads, if any, and
//! where each was found.
//!
//! A project and a vault are different things. A **project** is HQL's: a
//! directory holding `.hql/config.toml`, which supplies configuration,
//! packages and endpoints. A **vault** is HyperMarkdown's: a directory
//! holding `.hmd/`, whose documents are cards. Neither marks the other, and
//! the two are found independently.
//!
//! Five locations stay distinct: the working directory, where `hql` was
//! started; the anchor, the source directory discovery starts from; the
//! project root; the vault root; and the HQL home. Project discovery walks
//! upward from the anchor to the nearest directory holding
//! `.hql/config.toml`, skipping the directory whose `.hql/` is the HQL home;
//! vault discovery walks upward to the nearest `.hmd/`. An explicit
//! selection replaces either walk, and bare execution suppresses both.
//!
//! Bare means no project, not no configuration. The system configuration
//! file, the user's HQL home (`~/.hql`, or `$HQL_HOME`) and `$HQL_PATH`
//! contribute in every mode; a project's `.hql/config.toml` adds to them. A
//! relative path in the project file is read against the project root; in
//! the user or system file, against the directory holding that file.
//!
//! A named context is a `[context.<name>]` table of the system or user file:
//! reusable configuration, selected by `--context`, then `$HQL_CONTEXT`,
//! then `current-context`, then a context named `default`. A context is not
//! a project; it binds one only when it states `project`.

use std::path::{Path, PathBuf};

/// The directory that marks a HyperMarkdown vault.
pub const VAULT_MARKER: &str = ".hmd";
/// The directory that holds a project's configuration and state, and the
/// HQL home's own name under the user's home directory.
pub const PROJECT_DIRECTORY: &str = ".hql";
/// The configuration file inside `.hql/`, of a project and of the HQL home.
pub const CONFIG_NAME: &str = "config.toml";
/// A project's configuration file, relative to the project root.
pub const PROJECT_CONFIG: &str = ".hql/config.toml";
/// The environment variable naming the HQL home in place of `~/.hql`.
pub const HOME_ENVIRONMENT: &str = "HQL_HOME";
/// The environment variable naming the system configuration file.
pub const SYSTEM_ENVIRONMENT: &str = "HQL_SYSTEM_CONFIG";
/// The environment variable listing further module roots.
pub const PATH_ENVIRONMENT: &str = "HQL_PATH";
/// The environment variable selecting a named context for one invocation.
pub const CONTEXT_ENVIRONMENT: &str = "HQL_CONTEXT";
/// The key of the system or user file naming the current context.
pub const CURRENT_CONTEXT: &str = "current-context";
/// The context selected when nothing else selects one, if it is defined.
pub const DEFAULT_CONTEXT: &str = "default";
/// The HQL home's directory under the user's home directory.
pub const HOME_DIRECTORY: &str = PROJECT_DIRECTORY;
/// The configuration file of the HQL home and of the system.
pub const USER_CONFIG: &str = CONFIG_NAME;
/// The HQL home's directory of module files, a module root when present.
pub const MODULES_DIRECTORY: &str = "modules";

/// The system configuration file where the platform has one.
#[must_use]
pub fn default_system_config() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("ProgramData")
            .map(|data| PathBuf::from(data).join("hql").join(USER_CONFIG))
    } else {
        Some(PathBuf::from("/etc/hql").join(USER_CONFIG))
    }
}

/// Why an execution context could not be constructed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContextError {
    /// Two explicit selections contradict each other.
    #[error("{0}")]
    Conflict(String),
    /// An explicitly selected project or vault is not a directory.
    #[error("{}: not a directory", .0.display())]
    NotADirectory(PathBuf),
    /// An explicitly selected project holds no `.hql/config.toml`.
    #[error("{}: not a project: it holds no `{PROJECT_CONFIG}`", .0.display())]
    NotAProject(PathBuf),
    /// An explicitly selected vault holds no `.hmd/`.
    #[error("{}: not a vault: it holds no `{VAULT_MARKER}/`", .0.display())]
    NotAVault(PathBuf),
    /// A configuration file cannot be read, or says something malformed.
    #[error("{}: {message}", path.display())]
    Config {
        /// The file.
        path: PathBuf,
        /// What is wrong with it.
        message: String,
    },
    /// A selected context is defined by no configuration file.
    #[error(
        "no context is named `{name}`, which {selected_by} selects; {}",
        defined(known)
    )]
    UnknownContext {
        /// The selected name.
        name: String,
        /// The layer that selected it.
        selected_by: String,
        /// The names that are defined.
        known: Vec<String>,
    },
}

/// The defined context names, as a refusal lists them.
fn defined(known: &[String]) -> String {
    if known.is_empty() {
        return "no context is defined".to_owned();
    }
    let names: Vec<String> = known.iter().map(|name| format!("`{name}`")).collect();
    format!("the defined contexts are {}", names.join(", "))
}

/// A refusal naming the configuration file at fault.
fn refuse(path: &Path, message: impl Into<String>) -> ContextError {
    ContextError::Config {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

/// The layer a contribution to the context came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Nothing stated it.
    Default,
    /// The system configuration file.
    System,
    /// The user configuration file in the HQL home.
    User,
    /// The HQL home's own directories.
    Home,
    /// The selected named context.
    Context(String),
    /// The project's `.hql/config.toml`.
    Project,
    /// An environment variable, by name.
    Environment(&'static str),
    /// A command-line option, by name.
    Option(&'static str),
}

impl Origin {
    /// How `hql context` names the layer.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Default => "the default".to_owned(),
            Self::System => "system configuration".to_owned(),
            Self::User => "user configuration".to_owned(),
            Self::Home => "the HQL home".to_owned(),
            Self::Context(name) => format!("context `{name}`"),
            Self::Project => format!("the project's {PROJECT_CONFIG}"),
            Self::Environment(name) => format!("${name}"),
            Self::Option(name) => (*name).to_owned(),
        }
    }
}

/// A value the context holds, with the layer that supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contributed<T> {
    /// The value.
    pub value: T,
    /// Where it came from.
    pub origin: Origin,
}

/// Which file a table of configuration keys is read from, for the keys it
/// may hold (HQL-0059, part 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    /// A project's `.hql/config.toml`.
    Project,
    /// The user file or the system file.
    Layer,
    /// A `[context.<name>]` table of the user or system file.
    Context,
}

/// Check that a table holds only the keys its level takes.
fn validate(table: &toml::Table, level: Level, path: &Path) -> Result<(), ContextError> {
    for key in table.keys() {
        let known = match key.as_str() {
            "packages" | "modules" | "execution" | "report" => true,
            // Reserved for the AI harness (HQL-0022), specified there.
            "env" | "endpoint" | "keyvault" | "format" | "ai" => level == Level::Project,
            CURRENT_CONTEXT | "context" => level == Level::Layer,
            "project" => level == Level::Context,
            _ => false,
        };
        if !known {
            let file = match level {
                Level::Project => "a project's configuration",
                Level::Layer => "the user or system configuration",
                Level::Context => "a named context",
            };
            return Err(refuse(
                path,
                format!("`{key}` is not a key {file} takes; HQL-0059, part 1, lists them"),
            ));
        }
    }
    let section = |name: &str, keys: &[&str]| -> Result<(), ContextError> {
        let Some(value) = table.get(name) else {
            return Ok(());
        };
        let section = value
            .as_table()
            .ok_or_else(|| refuse(path, format!("`{name}` is a table, `[{name}]`")))?;
        if let Some(key) = section.keys().find(|key| !keys.contains(&key.as_str())) {
            let known: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
            return Err(refuse(
                path,
                format!("`[{name}]` takes no `{key}`; it takes {}", known.join(", ")),
            ));
        }
        Ok(())
    };
    section("modules", &["roots", "import", "prelude"])?;
    section("report", &["mode", "level"])?;
    match level {
        Level::Project => {
            section("packages", &["dependencies", "paths"])?;
            section("execution", &["backend", "max_elements", "max_bytes"])?;
        }
        Level::Layer => {
            section("packages", &["dependencies", "paths"])?;
            section("execution", &["backend"])?;
        }
        Level::Context => {
            section("packages", &["paths"])?;
            section("execution", &["backend"])?;
        }
    }
    Ok(())
}

/// One configuration file of the layering, and its contents when present.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigFile {
    /// Which layer it is.
    pub origin: Origin,
    /// Where it is, or would be.
    pub path: PathBuf,
    /// Its contents; `None` when there is no such file.
    pub table: Option<toml::Table>,
}

impl ConfigFile {
    /// Read a configuration file; an absent one is a layer with nothing in
    /// it. A present one must hold only the keys its layer takes.
    ///
    /// # Errors
    ///
    /// The file exists and cannot be read, is not TOML, or holds a key its
    /// layer does not take.
    pub fn read(origin: Origin, path: PathBuf) -> Result<Self, ContextError> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    origin,
                    path,
                    table: None,
                });
            }
            Err(error) => {
                return Err(ContextError::Config {
                    path,
                    message: error.to_string(),
                });
            }
        };
        let table = toml::from_str::<toml::Table>(&text).map_err(|error| ContextError::Config {
            path: path.clone(),
            message: error.message().to_owned(),
        })?;
        let level = if origin == Origin::Project {
            Level::Project
        } else {
            Level::Layer
        };
        validate(&table, level, &path)?;
        if let Some(contexts) = table.get("context").and_then(toml::Value::as_table) {
            for (name, entry) in contexts {
                if let Some(entry) = entry.as_table() {
                    validate(entry, Level::Context, &path).map_err(|error| match error {
                        ContextError::Config { path, message } => ContextError::Config {
                            path,
                            message: format!("`[context.{name}]`: {message}"),
                        },
                        other => other,
                    })?;
                }
            }
        }
        Ok(Self {
            origin,
            path,
            table: Some(table),
        })
    }

    /// Whether the file exists.
    #[must_use]
    pub fn is_present(&self) -> bool {
        self.table.is_some()
    }

    /// The directory a relative path in this file is read against: the
    /// project root for a project's `.hql/config.toml`, the directory
    /// holding the file otherwise.
    #[must_use]
    pub fn base(&self) -> &Path {
        let directory = self.path.parent().unwrap_or_else(|| Path::new("."));
        if self.origin == Origin::Project {
            directory.parent().unwrap_or(directory)
        } else {
            directory
        }
    }
}

/// The settings a configuration table contributes to the context: module
/// roots, implicit imports and the prelude. The same shape is read from
/// the system file, the user file, a named context and a project's file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Settings {
    module_roots: Vec<PathBuf>,
    imports: Vec<String>,
    prelude: Option<bool>,
}

impl Settings {
    /// Read the settings of `table`, whose paths are read against `base`;
    /// `path` names the declaring file in a refusal.
    fn of(table: &toml::Table, base: &Path, path: &Path) -> Result<Self, ContextError> {
        let modules = table.get("modules");
        let strings = |key: &str| -> Result<Vec<String>, ContextError> {
            let Some(value) = modules.and_then(|section| section.get(key)) else {
                return Ok(Vec::new());
            };
            value
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .map(|item| item.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| refuse(path, format!("`[modules] {key}` is a list of strings")))
        };
        let prelude = match modules.and_then(|section| section.get("prelude")) {
            None => None,
            Some(value) => Some(
                value
                    .as_bool()
                    .ok_or_else(|| refuse(path, "`[modules] prelude` is true or false"))?,
            ),
        };
        Ok(Self {
            module_roots: strings("roots")?
                .into_iter()
                .map(|root| absolute(base, Path::new(&root)))
                .collect(),
            imports: strings("import")?,
            prelude,
        })
    }
}

/// The user's HQL environment: `~/.hql`, or the directory `$HQL_HOME` names.
/// It is never a project, and never a vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HqlHome {
    /// The directory.
    pub path: PathBuf,
    /// `$HQL_HOME`, or the default under the user's home directory.
    pub origin: Origin,
}

impl HqlHome {
    /// The HQL home of an environment; none when neither `$HQL_HOME` nor a
    /// home directory is known.
    #[must_use]
    pub fn of(environment: &Environment) -> Option<Self> {
        if let Some(path) = &environment.hql_home {
            return Some(Self {
                path: absolute(&environment.cwd, path),
                origin: Origin::Environment(HOME_ENVIRONMENT),
            });
        }
        environment.home.as_ref().map(|home| Self {
            path: home.join(HOME_DIRECTORY),
            origin: Origin::Default,
        })
    }

    /// The user configuration file.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.path.join(USER_CONFIG)
    }

    /// Whether `dir` is the directory whose `.hql/` is this home: the one
    /// directory the project walk skips, since the user file has the shape
    /// of a project's.
    #[must_use]
    pub fn is_held_by(&self, dir: &Path) -> bool {
        let candidate = dir.join(PROJECT_DIRECTORY);
        candidate.is_dir() && same_directory(&candidate, &self.path)
    }
}

/// The configuration that exists independently of any project: the system
/// file and the user's HQL home.
#[derive(Debug, Clone, PartialEq)]
pub struct Layers {
    /// The HQL home, when one is known.
    pub home: Option<HqlHome>,
    /// The system configuration file, when the platform has one.
    pub system: Option<ConfigFile>,
    /// The user configuration file, in the HQL home.
    pub user: Option<ConfigFile>,
}

impl Layers {
    /// Read the system and user layers of an environment.
    ///
    /// # Errors
    ///
    /// A configuration file that exists and cannot be read as TOML, or that
    /// holds a key the user and system files do not take.
    pub fn load(environment: &Environment) -> Result<Self, ContextError> {
        let home = HqlHome::of(environment);
        let system = environment
            .system_config
            .as_ref()
            .map(|path| absolute(&environment.cwd, path))
            .or_else(default_system_config)
            .map(|path| ConfigFile::read(Origin::System, path))
            .transpose()?;
        let user = home
            .as_ref()
            .map(|home| ConfigFile::read(Origin::User, home.config_file()))
            .transpose()?;
        Ok(Self { home, system, user })
    }

    /// The system file, then the user file, where each is known.
    pub fn files(&self) -> impl Iterator<Item = &ConfigFile> {
        self.system.iter().chain(self.user.iter())
    }

    /// Every named context, by name. A user file's definition of a name
    /// replaces the system file's.
    ///
    /// # Errors
    ///
    /// A `context` entry that is not a table of tables.
    pub fn contexts(
        &self,
    ) -> Result<std::collections::BTreeMap<String, NamedContext>, ContextError> {
        let mut contexts = std::collections::BTreeMap::new();
        for file in self.files() {
            let Some(entries) = file.table.as_ref().and_then(|table| table.get("context")) else {
                continue;
            };
            let entries = entries
                .as_table()
                .ok_or_else(|| refuse(&file.path, "`context` is a table of named contexts"))?;
            for (name, entry) in entries {
                let table = entry
                    .as_table()
                    .ok_or_else(|| refuse(&file.path, format!("`[context.{name}]` is a table")))?;
                contexts.insert(
                    name.clone(),
                    NamedContext {
                        name: name.clone(),
                        origin: file.origin.clone(),
                        path: file.path.clone(),
                        table: table.clone(),
                    },
                );
            }
        }
        Ok(contexts)
    }

    /// The persistent `current-context`: the user file's, else the system
    /// file's.
    ///
    /// # Errors
    ///
    /// A `current-context` that is not a string.
    pub fn current_context(&self) -> Result<Option<Contributed<String>>, ContextError> {
        let mut current = None;
        for file in self.files() {
            let Some(value) = file
                .table
                .as_ref()
                .and_then(|table| table.get(CURRENT_CONTEXT))
            else {
                continue;
            };
            let name = value.as_str().ok_or_else(|| {
                refuse(&file.path, format!("`{CURRENT_CONTEXT}` is a context name"))
            })?;
            current = Some(Contributed {
                value: name.to_owned(),
                origin: file.origin.clone(),
            });
        }
        Ok(current)
    }

    /// The named context an invocation runs under: `--context`, then
    /// `$HQL_CONTEXT`, then `current-context`, then a context named
    /// `default` when one is defined. `None` when nothing selects one.
    ///
    /// # Errors
    ///
    /// The selected name is defined nowhere.
    pub fn select(
        &self,
        option: Option<&str>,
        environment: Option<&str>,
    ) -> Result<Option<(Contributed<String>, NamedContext)>, ContextError> {
        let contexts = self.contexts()?;
        let choice = if let Some(name) = option {
            Contributed {
                value: name.to_owned(),
                origin: Origin::Option("--context"),
            }
        } else if let Some(name) = environment {
            Contributed {
                value: name.to_owned(),
                origin: Origin::Environment(CONTEXT_ENVIRONMENT),
            }
        } else if let Some(current) = self.current_context()? {
            current
        } else if contexts.contains_key(DEFAULT_CONTEXT) {
            Contributed {
                value: DEFAULT_CONTEXT.to_owned(),
                origin: Origin::Default,
            }
        } else {
            return Ok(None);
        };
        let Some(named) = contexts.get(&choice.value).cloned() else {
            return Err(ContextError::UnknownContext {
                name: choice.value,
                selected_by: choice.origin.label(),
                known: contexts.into_keys().collect(),
            });
        };
        Ok(Some((choice, named)))
    }

    /// The system and user files merged as the layering reads them: a
    /// table merges key by key, and any other value of the user file
    /// replaces the system file's.
    #[must_use]
    pub fn merged(&self) -> toml::Table {
        let mut merged = toml::Table::new();
        for file in self.files() {
            if let Some(table) = &file.table {
                merge(&mut merged, table);
            }
        }
        merged
    }
}

/// Merge `over` into `base`, table by table.
fn merge(base: &mut toml::Table, over: &toml::Table) {
    for (key, value) in over {
        match (base.get_mut(key), value) {
            (Some(toml::Value::Table(held)), toml::Value::Table(table)) => merge(held, table),
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

/// A `[context.<name>]` table: settings grouped under a name, and the
/// project it binds when it states one.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedContext {
    /// Its name.
    pub name: String,
    /// The layer that defines it.
    pub origin: Origin,
    /// The file that defines it.
    pub path: PathBuf,
    /// Its settings.
    pub table: toml::Table,
}

impl NamedContext {
    /// The directory its relative paths are read against: its file's.
    #[must_use]
    pub fn base(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    /// The project it binds, if it states one.
    ///
    /// # Errors
    ///
    /// A `project` that is not a path.
    pub fn project(&self) -> Result<Option<PathBuf>, ContextError> {
        match self.table.get("project") {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(|path| Some(absolute(self.base(), &expand_home(path))))
                .ok_or_else(|| {
                    refuse(
                        &self.path,
                        format!("`[context.{}] project` is a path", self.name),
                    )
                }),
        }
    }
}

/// A path as written in a configuration file, with a leading `~/` read as
/// the user's home directory.
fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

/// A configuration file's text with `current-context` set to `name`: the
/// top-level key is replaced where it is written, or added as the first
/// line. Every other line, comments included, is kept as it was.
#[must_use]
pub fn set_current_context(text: &str, name: &str) -> String {
    let assignment = format!(
        "{CURRENT_CONTEXT} = {}",
        toml::Value::String(name.to_owned())
    );
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let top = lines
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .unwrap_or(lines.len());
    let written = lines[..top].iter().position(|line| {
        line.trim_start()
            .strip_prefix(CURRENT_CONTEXT)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    });
    match written {
        Some(index) => lines[index] = assignment,
        None => lines.insert(0, assignment),
    }
    let mut written = lines.join("\n");
    written.push('\n');
    written
}

/// The two execution modes an invocation runs in. Whether a vault is read
/// is a separate fact: a project may have one, and a bare run may too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// No project: only the system and user layers configure the run.
    Bare,
    /// A project's `.hql/config.toml` configures the run.
    Project,
}

impl ExecutionMode {
    /// The name `hql context` reports.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bare => "bare",
            Self::Project => "project",
        }
    }
}

/// How a project or a vault of an invocation was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The nearest marked directory at or above the anchor.
    Discovered,
    /// A command-line option, named as it was written.
    Option(&'static str),
    /// The selected named context binds it.
    Context(String),
    /// The embedding host supplied it.
    Host,
    /// The vault selected it: its `[hql] project`, or the nearest project
    /// at or above a vault named explicitly.
    Vault,
}

impl Selection {
    /// How `hql context` names the origin.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Discovered => "discovered from the anchor".to_owned(),
            Self::Vault => "selected by the vault".to_owned(),
            Self::Option(option) => (*option).to_owned(),
            Self::Context(name) => format!("context `{name}`"),
            Self::Host => "the host".to_owned(),
        }
    }

    /// Whether the selection is explicit rather than found by the walk.
    #[must_use]
    pub fn is_explicit(&self) -> bool {
        !matches!(self, Self::Discovered)
    }
}

/// A directory an invocation selected, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    /// The directory.
    pub root: PathBuf,
    /// How it was selected.
    pub origin: Selection,
}

impl Selected {
    /// The project configuration file of a project root.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.root.join(PROJECT_CONFIG)
    }
}

/// Whether a directory is a project root: it holds `.hql/config.toml`.
#[must_use]
pub fn is_project(dir: &Path) -> bool {
    dir.join(PROJECT_CONFIG).is_file()
}

/// Whether a directory is a vault root: it holds `.hmd/`.
#[must_use]
pub fn is_vault(dir: &Path) -> bool {
    dir.join(VAULT_MARKER).is_dir()
}

/// The nearest project at or above `anchor`, skipping the directory whose
/// `.hql/` is the HQL home.
#[must_use]
pub fn discover_project(anchor: &Path, home: Option<&HqlHome>) -> Option<PathBuf> {
    anchor
        .ancestors()
        .filter(|dir| !home.is_some_and(|home| home.is_held_by(dir)))
        .find(|dir| is_project(dir))
        .map(Path::to_path_buf)
}

/// The HQL project a vault selects with `[hql] project` in its
/// `.hmd/config.toml`, read against the vault root; `None` where it selects
/// none. HyperMarkdown owns the file; HQL reads this one table of it.
///
/// # Errors
///
/// The file is not TOML, `project` is not a path, or names a directory that
/// is not a project.
pub fn selected_project(vault: &Path) -> Result<Option<PathBuf>, ContextError> {
    let path = vault.join(VAULT_MARKER).join(CONFIG_NAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let table = toml::from_str::<toml::Table>(&text).map_err(|error| ContextError::Config {
        path: path.clone(),
        message: error.message().to_owned(),
    })?;
    let Some(value) = table.get("hql").and_then(|hql| hql.get("project")) else {
        return Ok(None);
    };
    let written = value
        .as_str()
        .ok_or_else(|| refuse(&path, "`[hql] project` is a path"))?;
    let root = absolute(vault, Path::new(written));
    if !is_project(&root) {
        return Err(refuse(
            &path,
            format!(
                "`[hql] project` names {}, which holds no `{PROJECT_CONFIG}`",
                root.display()
            ),
        ));
    }
    Ok(Some(root))
}

/// The project of a run that names none, as HQL-0059 resolves it: the one
/// the vault selects with `[hql] project`; else the nearest at or above the
/// anchor; else, for a vault named explicitly, the nearest at or above the
/// vault root, as the HyperMarkdown host of that vault finds it.
///
/// # Errors
///
/// A malformed `[hql] project`.
pub fn resolve_project(
    anchor: &Path,
    vault: Option<&Selected>,
    home: Option<&HqlHome>,
) -> Result<Option<Selected>, ContextError> {
    if let Some(vault) = vault
        && let Some(root) = selected_project(&vault.root)?
    {
        return Ok(Some(Selected {
            root,
            origin: Selection::Vault,
        }));
    }
    if let Some(root) = discover_project(anchor, home) {
        return Ok(Some(Selected {
            root,
            origin: Selection::Discovered,
        }));
    }
    if let Some(vault) = vault.filter(|vault| vault.origin.is_explicit())
        && let Some(root) = discover_project(&vault.root, home)
    {
        return Ok(Some(Selected {
            root,
            origin: Selection::Vault,
        }));
    }
    Ok(None)
}

/// The nearest vault at or above `anchor`: the walk HyperMarkdown makes.
#[must_use]
pub fn discover_vault(anchor: &Path) -> Option<PathBuf> {
    anchor
        .ancestors()
        .find(|dir| is_vault(dir))
        .map(Path::to_path_buf)
}

/// What the process supplies to context construction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// The process working directory.
    pub cwd: PathBuf,
    /// The user's home directory.
    pub home: Option<PathBuf>,
    /// `$HQL_HOME`.
    pub hql_home: Option<PathBuf>,
    /// `$HQL_SYSTEM_CONFIG`.
    pub system_config: Option<PathBuf>,
    /// `$HQL_PATH`, as the platform separates paths.
    pub hql_path: Option<std::ffi::OsString>,
    /// `$HQL_CONTEXT`.
    pub context: Option<String>,
}

impl Environment {
    /// The environment of this process.
    ///
    /// # Errors
    ///
    /// The working directory cannot be read.
    pub fn from_process() -> Result<Self, String> {
        let cwd = std::env::current_dir()
            .map_err(|error| format!("the working directory cannot be read: {error}"))?;
        let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        Ok(Self {
            cwd,
            home: std::env::home_dir(),
            hql_home: set(HOME_ENVIRONMENT).map(PathBuf::from),
            system_config: set(SYSTEM_ENVIRONMENT).map(PathBuf::from),
            hql_path: set(PATH_ENVIRONMENT),
            context: set(CONTEXT_ENVIRONMENT).and_then(|name| name.into_string().ok()),
        })
    }
}

/// What an invocation states explicitly: the command line's options, or what
/// a host such as HyperMarkdown supplies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The source directory discovery starts from; the working directory
    /// without one.
    pub anchor: Option<PathBuf>,
    /// Run with no project and no discovery, whatever the filesystem holds.
    pub bare: bool,
    /// A project named on the command line, `--project`.
    pub project: Option<PathBuf>,
    /// A vault named on the command line, `--vault`.
    pub vault: Option<PathBuf>,
    /// A project an embedding host supplies; discovery does not run.
    pub host_project: Option<PathBuf>,
    /// A vault an embedding host supplies; discovery does not run.
    pub host_vault: Option<PathBuf>,
    /// Module roots named on the command line, read against the working
    /// directory.
    pub modules: Vec<PathBuf>,
    /// Modules imported into every unit, named on the command line,
    /// `--import`: the command-line layer of `[modules] import`.
    pub imports: Vec<String>,
    /// A named context selected for this invocation alone.
    pub context: Option<String>,
}

impl Request {
    /// The anchor of a source file: the directory holding it. Standard input
    /// and an expression have none, and anchor at the working directory.
    #[must_use]
    pub fn anchor_of(file: &str) -> Option<PathBuf> {
        if file == "-" {
            return None;
        }
        Path::new(file)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
    }
}

/// The execution context an invocation runs in.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionContext {
    /// The process working directory.
    pub cwd: PathBuf,
    /// Where discovery starts: the source directory of the program.
    pub anchor: PathBuf,
    /// The project, absent in bare mode.
    pub project: Option<Selected>,
    /// The vault, when one is named, supplied or found.
    pub vault: Option<Selected>,
    /// Whether bare execution was requested, rather than found for want of
    /// a project.
    pub bare: bool,
    /// The environment it was constructed in.
    pub environment: Environment,
    /// The system and user layers.
    pub layers: Layers,
    /// The selected named context, and the layer that selected it.
    pub context: Option<Contributed<String>>,
    /// Its definition.
    pub named: Option<NamedContext>,
    /// The project's `.hql/config.toml`, in project mode.
    pub project_config: Option<ConfigFile>,
    /// Every module root, in layer order, each once.
    pub module_roots: Vec<Contributed<PathBuf>>,
    /// Every module imported into each unit, in layer order, each once.
    pub imports: Vec<Contributed<String>>,
    /// Whether the prelude's modules arrive unasked; the last layer stating
    /// it wins.
    pub prelude: Contributed<bool>,
}

impl ExecutionContext {
    /// Construct the context of a request in an environment.
    ///
    /// # Errors
    ///
    /// Contradictory explicit selections, an explicit project or vault that
    /// is not one, and a malformed configuration file.
    pub fn construct(request: &Request, environment: &Environment) -> Result<Self, ContextError> {
        let layers = Layers::load(environment)?;
        let anchor = request.anchor.as_ref().map_or_else(
            || environment.cwd.clone(),
            |anchor| absolute(&environment.cwd, anchor),
        );
        let selected = layers.select(request.context.as_deref(), environment.context.as_deref())?;
        let (context, named) = selected.unzip();
        // A context selected for this invocation is an explicit statement,
        // and contradicting it is an error; a persistent one yields.
        let chosen = context.as_ref().is_some_and(|choice| {
            matches!(choice.origin, Origin::Option(_) | Origin::Environment(_))
        });
        let bound = named
            .as_ref()
            .map(NamedContext::project)
            .transpose()?
            .flatten();
        let explicit = match (&request.host_project, &request.project) {
            (Some(_), Some(_)) => {
                return Err(ContextError::Conflict(
                    "the host supplies the project, and --project names another".to_owned(),
                ));
            }
            (Some(root), None) => Some(Selected {
                root: root.clone(),
                origin: Selection::Host,
            }),
            (None, Some(root)) => Some(Selected {
                root: root.clone(),
                origin: Selection::Option("--project"),
            }),
            (None, None) => None,
        };
        if request.bare
            && let Some(explicit) = &explicit
        {
            return Err(ContextError::Conflict(format!(
                "--bare runs with no project, and {} names one",
                explicit.origin.label()
            )));
        }
        if let (Some(choice), Some(root)) = (&context, &bound)
            && chosen
        {
            if request.bare {
                return Err(ContextError::Conflict(format!(
                    "--bare runs with no project, and context `{}`, which {} selects, binds {}",
                    choice.value,
                    choice.origin.label(),
                    root.display()
                )));
            }
            if let Some(explicit) = &explicit
                && !same_directory(&explicit.root, root)
            {
                return Err(ContextError::Conflict(format!(
                    "context `{}`, which {} selects, binds the project {}, and {} names {}",
                    choice.value,
                    choice.origin.label(),
                    root.display(),
                    explicit.origin.label(),
                    explicit.root.display()
                )));
            }
        }
        let explicit = explicit.or_else(|| {
            let name = context.as_ref()?.value.clone();
            bound.map(|root| Selected {
                root,
                origin: Selection::Context(name),
            })
        });
        let vault = match (&request.host_vault, &request.vault) {
            (Some(_), Some(_)) => {
                return Err(ContextError::Conflict(
                    "the host supplies the vault, and --vault names another".to_owned(),
                ));
            }
            (Some(root), None) => Some(Selected {
                root: root.clone(),
                origin: Selection::Host,
            }),
            (None, Some(root)) => {
                let root = root.clone();
                if !root.is_dir() {
                    return Err(ContextError::NotADirectory(root));
                }
                if !is_vault(&root) {
                    return Err(ContextError::NotAVault(root));
                }
                Some(Selected {
                    root,
                    origin: Selection::Option("--vault"),
                })
            }
            (None, None) if request.bare => None,
            (None, None) => discover_vault(&anchor).map(|root| Selected {
                root,
                origin: Selection::Discovered,
            }),
        };
        let project = if request.bare {
            None
        } else if let Some(explicit) = explicit {
            if !explicit.root.is_dir() {
                return Err(ContextError::NotADirectory(explicit.root));
            }
            if explicit.origin != Selection::Host && !is_project(&explicit.root) {
                return Err(ContextError::NotAProject(explicit.root));
            }
            Some(explicit)
        } else {
            resolve_project(&anchor, vault.as_ref(), layers.home.as_ref())?
        };
        let project_config = project
            .as_ref()
            .filter(|project| is_project(&project.root))
            .map(|project| ConfigFile::read(Origin::Project, project.config_file()))
            .transpose()?;
        let mut context = Self {
            cwd: environment.cwd.clone(),
            anchor,
            project,
            vault,
            bare: request.bare,
            environment: environment.clone(),
            layers,
            context,
            named,
            project_config,
            module_roots: Vec::new(),
            imports: Vec::new(),
            prelude: Contributed {
                value: true,
                origin: Origin::Default,
            },
        };
        context.contribute(request)?;
        Ok(context)
    }

    /// Gather the module roots and module settings of every layer, in
    /// order: the system file, the user file, the selected context, the HQL
    /// home's modules, the project's file, `$HQL_PATH`, then `--modules`
    /// and `--import`.
    fn contribute(&mut self, request: &Request) -> Result<(), ContextError> {
        let layers: Vec<ConfigFile> = self.layers.files().cloned().collect();
        for file in &layers {
            self.take(file)?;
        }
        if let Some(named) = self.named.clone() {
            let settings = Settings::of(&named.table, named.base(), &named.path)?;
            self.settle(settings, &Origin::Context(named.name));
        }
        if let Some(home) = &self.layers.home {
            let modules = home.path.join(MODULES_DIRECTORY);
            if modules.is_dir() {
                self.add_root(modules, Origin::Home);
            }
        }
        if let Some(file) = self.project_config.clone() {
            self.take(&file)?;
        }
        if let Some(paths) = self.environment.hql_path.clone() {
            for path in std::env::split_paths(&paths) {
                if !path.as_os_str().is_empty() {
                    self.add_root(path, Origin::Environment(PATH_ENVIRONMENT));
                }
            }
        }
        for path in &request.modules {
            self.add_root(path.clone(), Origin::Option("--modules"));
        }
        self.settle(
            Settings {
                imports: request.imports.clone(),
                ..Settings::default()
            },
            &Origin::Option("--import"),
        );
        Ok(())
    }

    /// Take the settings one configuration file states.
    fn take(&mut self, file: &ConfigFile) -> Result<(), ContextError> {
        let Some(table) = &file.table else {
            return Ok(());
        };
        let settings = Settings::of(table, file.base(), &file.path)?;
        self.settle(settings, &file.origin);
        Ok(())
    }

    /// Take one layer's settings.
    fn settle(&mut self, settings: Settings, origin: &Origin) {
        for root in settings.module_roots {
            self.add_root(root, origin.clone());
        }
        for import in settings.imports {
            if !self.imports.iter().any(|held| held.value == import) {
                self.imports.push(Contributed {
                    value: import,
                    origin: origin.clone(),
                });
            }
        }
        if let Some(prelude) = settings.prelude {
            self.prelude = Contributed {
                value: prelude,
                origin: origin.clone(),
            };
        }
    }

    /// Add a module root unless it is already one: the same directory named
    /// twice is one root, not two providers of every module under it.
    fn add_root(&mut self, path: PathBuf, origin: Origin) {
        if self
            .module_roots
            .iter()
            .any(|held| same_directory(&held.value, &path))
        {
            return;
        }
        self.module_roots.push(Contributed {
            value: path,
            origin,
        });
    }

    /// Hand the effective module roots and module settings to the
    /// extension configuration a run carries.
    pub fn configure(&self, config: &mut crate::extensions::Config) {
        config.module_roots = self
            .module_roots
            .iter()
            .map(|root| root.value.clone())
            .collect();
        config.imports = self
            .imports
            .iter()
            .map(|import| import.value.clone())
            .collect();
        config.prelude = self.prelude.value;
    }

    /// The environment variables the project grants a program: the names
    /// and `PREFIX_*` patterns of `env = [...]` in its `.hql/config.toml`,
    /// or `None` where it states none and every variable is granted.
    ///
    /// # Errors
    ///
    /// `env` is stated and is not a list of strings.
    pub fn env_grant(&self) -> Result<Option<Vec<String>>, ContextError> {
        let Some(file) = &self.project_config else {
            return Ok(None);
        };
        let Some(value) = file.table.as_ref().and_then(|table| table.get("env")) else {
            return Ok(None);
        };
        value
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .map(Some)
            .ok_or_else(|| {
                refuse(
                    &file.path,
                    "`env` is a list of variable names and `PREFIX_*` patterns",
                )
            })
    }

    /// A scalar setting `[section] key` that the project, the environment
    /// and the command line leave to configuration: the selected context's,
    /// else the user file's, else the system file's.
    ///
    /// # Errors
    ///
    /// The setting is stated and is not a string.
    pub fn setting(
        &self,
        section: &str,
        key: &str,
    ) -> Result<Option<Contributed<String>>, ContextError> {
        let named = self.named.iter().map(|named| {
            (
                &named.table,
                &named.path,
                Origin::Context(named.name.clone()),
            )
        });
        let files = self
            .layers
            .user
            .iter()
            .chain(self.layers.system.iter())
            .filter_map(|file| {
                file.table
                    .as_ref()
                    .map(|table| (table, &file.path, file.origin.clone()))
            });
        for (table, path, origin) in named.chain(files) {
            let Some(value) = table.get(section).and_then(|section| section.get(key)) else {
                continue;
            };
            let value = value
                .as_str()
                .ok_or_else(|| refuse(path, format!("`[{section}] {key}` is a string")))?;
            return Ok(Some(Contributed {
                value: value.to_owned(),
                origin,
            }));
        }
        Ok(None)
    }

    /// Load what the run reads: the vault's documents as cards, when there
    /// is a vault, under the project's configuration, when there is a
    /// project, and the module settings of every layer. A host that supplies
    /// the project or the vault calls this after [`Self::construct`].
    ///
    /// # Errors
    ///
    /// The vault cannot be read, or the project's configuration is
    /// malformed, naming the directory.
    pub fn load(&self) -> Result<crate::environment::RunEnvironment, String> {
        let mut env = crate::environment::RunEnvironment::bare();
        if let Some(root) = self.vault() {
            env.vault = Some(
                crate::vault::load_documents(root)
                    .map_err(|error| format!("{}: {error}", root.display()))?,
            );
        }
        if let Some(root) = self.project_root() {
            env.configure_project(root)
                .map_err(|error| format!("{}: {error}", root.display()))?;
        }
        self.configure(&mut env.extensions);
        // A run with a vault, found by the walk or named, imports `hmd`: the
        // vault is what `hmd` reads, so its presence activates the package.
        if env.has_vault() {
            env.host();
        }
        Ok(env)
    }

    /// Every configuration file of the layering, in order, present or not.
    pub fn config_files(&self) -> impl Iterator<Item = &ConfigFile> {
        self.layers.files().chain(self.project_config.iter())
    }

    /// The mode the context puts the invocation in.
    #[must_use]
    pub fn mode(&self) -> ExecutionMode {
        if self.project.is_some() {
            ExecutionMode::Project
        } else {
            ExecutionMode::Bare
        }
    }

    /// The vault root, when there is a vault.
    #[must_use]
    pub fn vault(&self) -> Option<&Path> {
        self.vault.as_ref().map(|vault| vault.root.as_path())
    }

    /// The project root, absent in bare mode.
    #[must_use]
    pub fn project_root(&self) -> Option<&Path> {
        self.project.as_ref().map(|project| project.root.as_path())
    }
}

/// Whether two paths name one directory.
fn same_directory(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

/// A path as given when absolute, else joined to `base`, with `.` and `..`
/// resolved by position. The walk upward then visits only the directory's
/// real ancestors: `/a/tmp/../notes` walks `/a/notes`, then `/a`, and never
/// `/a/tmp`.
pub(crate) fn absolute(base: &Path, path: &Path) -> PathBuf {
    use std::path::Component;
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let mut normal = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normal.pop() {
                    normal.push(component);
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normal.push(component);
            }
        }
    }
    normal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn environment(cwd: &Path) -> Environment {
        Environment {
            cwd: cwd.to_path_buf(),
            system_config: Some(cwd.join("no-system-config.toml")),
            ..Environment::default()
        }
    }

    fn anchored(anchor: &Path) -> Request {
        Request {
            anchor: Some(anchor.to_path_buf()),
            ..Request::default()
        }
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Mark `dir` as a project with an empty `.hql/config.toml`.
    fn project(dir: &Path) {
        write(&dir.join(PROJECT_CONFIG), "");
    }

    #[test]
    fn a_directory_with_no_project_above_it_runs_bare() {
        let root = tempfile::tempdir().unwrap();
        let context =
            ExecutionContext::construct(&Request::default(), &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert_eq!(context.project, None);
        assert_eq!(context.vault(), None);
        assert_eq!(context.anchor, root.path());
    }

    #[test]
    fn the_walks_start_at_the_anchor_not_the_working_directory() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("a");
        project(&repo);
        fs::create_dir_all(repo.join("b/.hmd")).unwrap();
        fs::create_dir_all(repo.join("b/c")).unwrap();
        let elsewhere = root.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let context =
            ExecutionContext::construct(&anchored(&repo.join("b/c")), &environment(&elsewhere))
                .unwrap();
        assert_eq!(context.mode(), ExecutionMode::Project);
        assert_eq!(context.project_root(), Some(repo.as_path()));
        assert_eq!(context.vault(), Some(repo.join("b").as_path()));
        assert_eq!(context.cwd, elsewhere);
        assert_eq!(context.anchor, repo.join("b/c"));
    }

    #[test]
    fn a_vault_is_not_a_project_and_a_project_is_not_a_vault() {
        let root = tempfile::tempdir().unwrap();
        let vault = root.path().join("vault");
        fs::create_dir_all(vault.join(".hmd")).unwrap();
        let context =
            ExecutionContext::construct(&anchored(&vault), &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert_eq!(context.vault(), Some(vault.as_path()));
        let repo = root.path().join("repo");
        project(&repo);
        let context =
            ExecutionContext::construct(&anchored(&repo), &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Project);
        assert_eq!(context.vault(), None);
    }

    #[test]
    fn a_relative_anchor_is_read_against_the_working_directory() {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        fs::create_dir_all(root.path().join("src")).unwrap();
        let context =
            ExecutionContext::construct(&anchored(Path::new("src")), &environment(root.path()))
                .unwrap();
        assert_eq!(context.anchor, root.path().join("src"));
        assert_eq!(context.project_root(), Some(root.path()));
    }

    #[test]
    fn an_anchor_through_a_parent_walks_only_its_real_ancestors() {
        let root = tempfile::tempdir().unwrap();
        let tmp = root.path().join("tmp");
        project(&tmp);
        fs::create_dir_all(root.path().join("notes/scripts")).unwrap();
        let context = ExecutionContext::construct(
            &anchored(Path::new("../notes/scripts")),
            &environment(&tmp),
        )
        .unwrap();
        assert_eq!(context.anchor, root.path().join("notes/scripts"));
        assert_eq!(context.mode(), ExecutionMode::Bare);
    }

    #[test]
    fn the_nearest_project_wins() {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        let inner = root.path().join("inner");
        project(&inner);
        fs::create_dir_all(inner.join("src")).unwrap();
        let context =
            ExecutionContext::construct(&anchored(&inner.join("src")), &environment(root.path()))
                .unwrap();
        assert_eq!(context.project_root(), Some(inner.as_path()));
    }

    #[test]
    fn the_hql_home_is_never_a_project() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), "");
        fs::create_dir_all(root.join("home/code/repo")).unwrap();
        let environment = Environment {
            cwd: root.join("home/code/repo"),
            home: Some(root.join("home")),
            system_config: Some(root.join("etc/config.toml")),
            ..Environment::default()
        };
        let context = ExecutionContext::construct(&Request::default(), &environment).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert!(context.layers.user.as_ref().unwrap().is_present());
        // A project inside the home directory is found as any other is.
        project(&root.join("home/code"));
        let context = ExecutionContext::construct(&Request::default(), &environment).unwrap();
        assert_eq!(
            context.project_root(),
            Some(root.join("home/code").as_path())
        );
    }

    #[test]
    fn bare_suppresses_both_walks() {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        fs::create_dir_all(root.path().join(".hmd")).unwrap();
        let request = Request {
            bare: true,
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert_eq!(context.vault(), None);
    }

    #[test]
    fn an_explicit_project_replaces_the_walk_and_keeps_the_anchor() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        project(&repo);
        let nested = repo.join("src/jobs");
        project(&nested);
        let request = Request {
            anchor: Some(nested.clone()),
            project: Some(repo.clone()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(root.path())).unwrap();
        assert_eq!(context.project_root(), Some(repo.as_path()));
        assert_eq!(context.anchor, nested);
        assert_eq!(
            context.project.unwrap().origin,
            Selection::Option("--project")
        );
    }

    #[test]
    fn an_explicit_project_or_vault_must_be_marked() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        let with = |project: Option<PathBuf>, vault: Option<PathBuf>| {
            ExecutionContext::construct(
                &Request {
                    project,
                    vault,
                    ..Request::default()
                },
                &environment(root.path()),
            )
        };
        assert_eq!(
            with(Some(missing.clone()), None),
            Err(ContextError::NotADirectory(missing))
        );
        assert_eq!(
            with(Some(root.path().to_path_buf()), None),
            Err(ContextError::NotAProject(root.path().to_path_buf()))
        );
        assert_eq!(
            with(None, Some(root.path().to_path_buf())),
            Err(ContextError::NotAVault(root.path().to_path_buf()))
        );
    }

    #[test]
    fn bare_with_a_project_is_a_conflict() {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        let request = Request {
            bare: true,
            project: Some(root.path().to_path_buf()),
            ..Request::default()
        };
        let refused = ExecutionContext::construct(&request, &environment(root.path())).unwrap_err();
        assert!(matches!(refused, ContextError::Conflict(_)), "{refused}");
        assert!(refused.to_string().contains("--bare"), "{refused}");
    }

    #[test]
    fn a_host_project_and_vault_are_authoritative_and_need_no_marker() {
        let root = tempfile::tempdir().unwrap();
        let outer = root.path();
        project(outer);
        fs::create_dir_all(outer.join(".hmd")).unwrap();
        let hosted = outer.join("hosted");
        fs::create_dir_all(&hosted).unwrap();
        let request = Request {
            host_project: Some(hosted.clone()),
            host_vault: Some(hosted.clone()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(outer)).unwrap();
        assert_eq!(context.project_root(), Some(hosted.as_path()));
        assert_eq!(context.vault(), Some(hosted.as_path()));
        assert_eq!(context.project.unwrap().origin, Selection::Host);
        let both = Request {
            host_project: Some(hosted.clone()),
            project: Some(hosted),
            ..Request::default()
        };
        assert!(matches!(
            ExecutionContext::construct(&both, &environment(outer)),
            Err(ContextError::Conflict(_))
        ));
    }

    /// An environment with a user home, an HQL home under it and a system
    /// file, all inside `root`.
    fn configured(root: &Path) -> Environment {
        Environment {
            cwd: root.join("tree"),
            home: Some(root.join("home")),
            system_config: Some(root.join("etc/config.toml")),
            ..Environment::default()
        }
    }

    fn roots(context: &ExecutionContext) -> Vec<(PathBuf, Origin)> {
        context
            .module_roots
            .iter()
            .map(|root| (root.value.clone(), root.origin.clone()))
            .collect()
    }

    #[test]
    fn bare_mode_still_reads_the_system_file_the_hql_home_and_hql_path() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        project(&root.join("tree"));
        write(
            &root.join("etc/config.toml"),
            "[modules]\nroots = [\"lib\"]\nimport = [\"hmd\"]\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[modules]\nroots = [\"mine\"]\nprelude = false\nimport = [\"hmd\", \"net\"]\n",
        );
        fs::create_dir_all(root.join("home/.hql/modules")).unwrap();
        let mut environment = configured(root);
        environment.hql_path =
            Some(std::env::join_paths([root.join("path/a"), root.join("path/b")]).unwrap());
        let request = Request {
            bare: true,
            modules: vec![PathBuf::from("cli")],
            imports: vec!["stats".to_owned()],
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert_eq!(
            roots(&context),
            vec![
                (root.join("etc/lib"), Origin::System),
                (root.join("home/.hql/mine"), Origin::User),
                (root.join("home/.hql/modules"), Origin::Home),
                (root.join("path/a"), Origin::Environment(PATH_ENVIRONMENT)),
                (root.join("path/b"), Origin::Environment(PATH_ENVIRONMENT)),
                (PathBuf::from("cli"), Origin::Option("--modules")),
            ]
        );
        let imports: Vec<_> = context
            .imports
            .iter()
            .map(|import| (import.value.as_str(), import.origin.clone()))
            .collect();
        assert_eq!(
            imports,
            vec![
                ("hmd", Origin::System),
                ("net", Origin::User),
                ("stats", Origin::Option("--import"))
            ]
        );
        assert_eq!(
            context.prelude,
            Contributed {
                value: false,
                origin: Origin::User
            }
        );
        let home = context.layers.home.as_ref().unwrap();
        assert_eq!(home.path, root.join("home/.hql"));
        assert_eq!(home.origin, Origin::Default);
    }

    #[test]
    fn a_project_adds_its_own_roots_read_against_the_project_root() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("tree/.hql/config.toml"),
            "[modules]\nroots = [\"./lib\"]\nprelude = true\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[modules]\nprelude = false\n",
        );
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            roots(&context),
            vec![(root.join("tree/lib"), Origin::Project)]
        );
        assert_eq!(context.prelude.origin, Origin::Project);
        assert!(context.prelude.value);
        let files: Vec<_> = context
            .config_files()
            .map(|file| (file.origin.clone(), file.is_present()))
            .collect();
        assert_eq!(
            files,
            vec![
                (Origin::System, false),
                (Origin::User, true),
                (Origin::Project, true)
            ]
        );
    }

    #[test]
    fn hql_home_replaces_the_default_and_one_directory_is_one_root() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        fs::create_dir_all(root.join("elsewhere/modules")).unwrap();
        let mut environment = configured(root);
        environment.hql_home = Some(root.join("elsewhere"));
        environment.hql_path = Some(root.join("elsewhere/modules").into_os_string());
        let context = ExecutionContext::construct(&Request::default(), &environment).unwrap();
        let home = context.layers.home.as_ref().unwrap();
        assert_eq!(home.origin, Origin::Environment(HOME_ENVIRONMENT));
        assert_eq!(
            roots(&context),
            vec![(root.join("elsewhere/modules"), Origin::Home)]
        );
    }

    #[test]
    fn a_malformed_configuration_file_is_refused_naming_it() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("home/.hql/config.toml"),
            "[modules]\nroots = \"lib\"\n",
        );
        let refused =
            ExecutionContext::construct(&Request::default(), &configured(root)).unwrap_err();
        assert!(refused.to_string().contains("config.toml"), "{refused}");
        assert!(refused.to_string().contains("list of strings"), "{refused}");
        write(&root.join("home/.hql/config.toml"), "[modules\n");
        assert!(matches!(
            ExecutionContext::construct(&Request::default(), &configured(root)),
            Err(ContextError::Config { .. })
        ));
    }

    #[test]
    fn an_unknown_key_is_refused_naming_it() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        let refusal = |file: &str, text: &str| {
            write(&root.join(file), text);
            let refused = ExecutionContext::construct(&Request::default(), &configured(root))
                .unwrap_err()
                .to_string();
            fs::remove_file(root.join(file)).unwrap();
            refused
        };
        let refused = refusal("tree/.hql/config.toml", "[semantics]\nindex = \"i\"\n");
        assert!(refused.contains("`semantics`"), "{refused}");
        let refused = refusal("tree/.hql/config.toml", "[sources.db]\nkind = \"csv\"\n");
        assert!(refused.contains("`sources`"), "{refused}");
        let refused = refusal("tree/.hql/config.toml", "[nonsense]\n");
        assert!(refused.contains("`nonsense`"), "{refused}");
        let refused = refusal("tree/.hql/config.toml", "current-context = \"a\"\n");
        assert!(refused.contains(CURRENT_CONTEXT), "{refused}");
        let refused = refusal("home/.hql/config.toml", "[endpoint.a]\nkind = \"csv\"\n");
        assert!(refused.contains("`endpoint`"), "{refused}");
        let refused = refusal("home/.hql/config.toml", "[context.a]\nworkspace = \"/x\"\n");
        assert!(refused.contains("`workspace`"), "{refused}");
        let refused = refusal("tree/.hql/config.toml", "[modules]\nrots = []\n");
        assert!(refused.contains("`rots`"), "{refused}");
    }

    const CONTEXTS: &str = "\
current-context = \"work\"

[context.work.modules]
roots = [\"work-lib\"]

[context.experimental.modules]
prelude = false

[context.production]
project = \"../srv/production\"
";

    fn selected(context: &ExecutionContext) -> Option<(String, Origin)> {
        context
            .context
            .as_ref()
            .map(|choice| (choice.value.clone(), choice.origin.clone()))
    }

    #[test]
    fn a_context_is_selected_by_option_then_environment_then_current_context() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        let mut environment = configured(root);
        let context = ExecutionContext::construct(&Request::default(), &environment).unwrap();
        assert_eq!(selected(&context), Some(("work".to_owned(), Origin::User)));
        assert_eq!(
            roots(&context),
            vec![(
                root.join("home/.hql/work-lib"),
                Origin::Context("work".to_owned())
            )]
        );

        environment.context = Some("experimental".to_owned());
        let context = ExecutionContext::construct(&Request::default(), &environment).unwrap();
        assert_eq!(
            selected(&context),
            Some((
                "experimental".to_owned(),
                Origin::Environment(CONTEXT_ENVIRONMENT)
            ))
        );
        assert_eq!(
            context.prelude,
            Contributed {
                value: false,
                origin: Origin::Context("experimental".to_owned())
            }
        );

        let request = Request {
            context: Some("work".to_owned()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment).unwrap();
        assert_eq!(
            selected(&context),
            Some(("work".to_owned(), Origin::Option("--context")))
        );
    }

    #[test]
    fn an_undefined_context_is_refused_listing_the_defined_ones() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        let request = Request {
            context: Some("staging".to_owned()),
            ..Request::default()
        };
        let refused = ExecutionContext::construct(&request, &configured(root)).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "no context is named `staging`, which --context selects; the defined contexts are \
             `experimental`, `production`, `work`"
        );
    }

    #[test]
    fn a_context_named_default_applies_when_nothing_selects_another() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(selected(&context), None);
        write(
            &root.join("etc/config.toml"),
            "[context.default.modules]\nimport = [\"hmd\"]\n",
        );
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            selected(&context),
            Some((DEFAULT_CONTEXT.to_owned(), Origin::Default))
        );
        assert_eq!(context.imports[0].value, "hmd");
        assert_eq!(
            context.imports[0].origin,
            Origin::Context(DEFAULT_CONTEXT.to_owned())
        );
    }

    #[test]
    fn a_user_definition_replaces_the_system_one_of_the_same_name() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("etc/config.toml"),
            "current-context = \"shared\"\n[context.shared.modules]\nroots = [\"system-lib\"]\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[context.shared.modules]\nroots = [\"user-lib\"]\n",
        );
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            selected(&context),
            Some(("shared".to_owned(), Origin::System))
        );
        assert_eq!(
            roots(&context),
            vec![(
                root.join("home/.hql/user-lib"),
                Origin::Context("shared".to_owned())
            )]
        );
    }

    #[test]
    fn a_context_that_binds_a_project_replaces_discovery() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        let production = root.join("home/srv/production");
        project(&production);
        project(&root.join("tree"));
        let request = Request {
            context: Some("production".to_owned()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &configured(root)).unwrap();
        let bound = context.project.as_ref().unwrap();
        assert!(same_directory(&bound.root, &production));
        assert_eq!(bound.origin, Selection::Context("production".to_owned()));

        // The same context reused without a binding leaves discovery alone.
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(context.project.unwrap().origin, Selection::Discovered);
    }

    #[test]
    fn contradicting_a_selected_context_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        project(&root.join("home/srv/production"));
        project(&root.join("tree"));
        let production = || Some("production".to_owned());
        for request in [
            Request {
                context: production(),
                bare: true,
                ..Request::default()
            },
            Request {
                context: production(),
                project: Some(root.join("tree")),
                ..Request::default()
            },
        ] {
            let refused = ExecutionContext::construct(&request, &configured(root)).unwrap_err();
            assert!(matches!(refused, ContextError::Conflict(_)), "{refused}");
            assert!(refused.to_string().contains("production"), "{refused}");
        }
        // Naming the project the context binds is no contradiction.
        let agreeing = Request {
            context: production(),
            project: Some(root.join("home/srv/production")),
            ..Request::default()
        };
        assert!(ExecutionContext::construct(&agreeing, &configured(root)).is_ok());
    }

    #[test]
    fn a_persistent_context_yields_to_an_explicit_project_or_bare() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("home/.hql/config.toml"),
            &CONTEXTS.replace(
                "current-context = \"work\"",
                "current-context = \"production\"",
            ),
        );
        project(&root.join("home/srv/production"));
        project(&root.join("tree"));
        let bare = Request {
            bare: true,
            ..Request::default()
        };
        let context = ExecutionContext::construct(&bare, &configured(root)).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        let named = Request {
            project: Some(root.join("tree")),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&named, &configured(root)).unwrap();
        assert_eq!(context.project_root(), Some(root.join("tree").as_path()));
    }

    #[test]
    fn setting_the_current_context_keeps_every_other_line() {
        assert_eq!(
            set_current_context("", "work"),
            "current-context = \"work\"\n"
        );
        assert_eq!(
            set_current_context(
                "# mine\ncurrent-context = \"old\"\n[context.work]\n",
                "work"
            ),
            "# mine\ncurrent-context = \"work\"\n[context.work]\n"
        );
        let added = set_current_context("[context.work]\ncurrent-context = 1\n", "work");
        assert_eq!(
            added,
            "current-context = \"work\"\n[context.work]\ncurrent-context = 1\n"
        );
        let parsed: toml::Table = toml::from_str(&added).unwrap();
        assert_eq!(parsed[CURRENT_CONTEXT].as_str(), Some("work"));
    }

    #[test]
    fn the_merged_view_takes_the_user_value_over_the_system_one() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("etc/config.toml"),
            "current-context = \"a\"\n[context.a]\nproject = \"/x\"\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "current-context = \"b\"\n[context.b]\n",
        );
        let layers = Layers::load(&configured(root)).unwrap();
        let merged = layers.merged();
        assert_eq!(merged[CURRENT_CONTEXT].as_str(), Some("b"));
        let contexts = merged["context"].as_table().unwrap();
        assert_eq!(contexts.keys().collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn a_setting_is_the_contexts_then_the_users_then_the_systems() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("etc/config.toml"),
            "[report]\nmode = \"collect\"\nlevel = \"info\"\n[execution]\nbackend = \"x\"\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[report]\nlevel = \"error\"\n[context.loud.report]\nlevel = \"debug\"\n",
        );
        let stated = |context: &ExecutionContext, section: &str, key: &str| {
            context
                .setting(section, key)
                .unwrap()
                .map(|stated| (stated.value, stated.origin))
        };
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            stated(&context, "report", "mode"),
            Some(("collect".to_owned(), Origin::System))
        );
        assert_eq!(
            stated(&context, "report", "level"),
            Some(("error".to_owned(), Origin::User))
        );
        assert_eq!(stated(&context, "nothing", "here"), None);
        let request = Request {
            context: Some("loud".to_owned()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &configured(root)).unwrap();
        assert_eq!(
            stated(&context, "report", "level"),
            Some(("debug".to_owned(), Origin::Context("loud".to_owned())))
        );
    }
}
