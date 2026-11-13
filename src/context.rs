//! The execution context of an invocation (HQL-0025): whether it runs bare or
//! in a workspace, which workspace, and where it was found.
//!
//! Four locations stay distinct: the process working directory, the
//! execution anchor that discovery starts from, the workspace root, and the
//! vault root. Discovery walks upward from the anchor to the nearest
//! directory marked by `.hmd/` or `hql.toml`; an explicit selection replaces
//! the walk, and bare execution suppresses it. Module discovery, extension
//! loading and the document loader consume the result; none of them is
//! decided here.
//!
//! Bare means no workspace, not no configuration. The system configuration
//! file, the user's HQL home (`~/.hql`, or `$HQL_HOME`) and `$HQL_PATH`
//! contribute in every mode; a workspace's `hql.toml` adds to them. A
//! relative path in a configuration file is read against the directory of
//! the file that declares it.
//!
//! A named context is a `[context.<name>]` table of the system or user file:
//! reusable configuration, selected by `--context`, then `$HQL_CONTEXT`,
//! then `current-context`, then a context named `default`. A context is not
//! a workspace; it binds one only when it states `workspace`.

use crate::reporting::CONFIG_FILE;
use std::path::{Path, PathBuf};

/// The directory that marks a HyperMarkdown vault.
pub const VAULT_MARKER: &str = ".hmd";
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
pub const HOME_DIRECTORY: &str = ".hql";
/// The configuration file of the HQL home and of the system.
pub const USER_CONFIG: &str = "config.toml";
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
    /// An explicitly selected workspace is not a directory.
    #[error("{}: not a directory", .0.display())]
    NotADirectory(PathBuf),
    /// An explicitly selected workspace carries neither marker.
    #[error("{}: not a workspace: it holds neither `.hmd/` nor `hql.toml`", .0.display())]
    NotAWorkspace(PathBuf),
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
    /// The workspace's `hql.toml`.
    Workspace,
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
            Self::Workspace => format!("the workspace's {CONFIG_FILE}"),
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
    /// Read a configuration file; an absent one is a layer with nothing in it.
    ///
    /// # Errors
    ///
    /// The file exists and cannot be read, or is not TOML.
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
        match toml::from_str::<toml::Table>(&text) {
            Ok(table) => Ok(Self {
                origin,
                path,
                table: Some(table),
            }),
            Err(error) => Err(ContextError::Config {
                path,
                message: error.message().to_owned(),
            }),
        }
    }

    /// Whether the file exists.
    #[must_use]
    pub fn is_present(&self) -> bool {
        self.table.is_some()
    }

    /// The directory a relative path in this file is read against.
    #[must_use]
    pub fn base(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }
}

/// The settings a configuration table contributes to the context: module
/// roots, extension imports and the prelude. The same shape is read from
/// the system file, the user file and a workspace's `hql.toml`.
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
        let strings = |section: &str, key: &str| -> Result<Vec<String>, ContextError> {
            let Some(value) = table.get(section).and_then(|section| section.get(key)) else {
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
                .ok_or_else(|| refuse(path, format!("`[{section}] {key}` is a list of strings")))
        };
        let prelude = match table
            .get("extensions")
            .and_then(|section| section.get("prelude"))
        {
            None => None,
            Some(value) => Some(
                value
                    .as_bool()
                    .ok_or_else(|| refuse(path, "`[extensions] prelude` is true or false"))?,
            ),
        };
        Ok(Self {
            module_roots: strings("modules", "roots")?
                .into_iter()
                .map(|root| absolute(base, Path::new(&root)))
                .collect(),
            imports: strings("extensions", "import")?,
            prelude,
        })
    }
}

/// The user's HQL environment: `~/.hql`, or the directory `$HQL_HOME` names.
/// It is not a workspace, and never a vault.
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
}

/// The configuration that exists independently of any workspace: the
/// system file and the user's HQL home.
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
    /// A configuration file that exists and cannot be read as TOML.
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
/// workspace it binds when it states one.
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

    /// The workspace it binds, if it states one.
    ///
    /// # Errors
    ///
    /// A `workspace` that is not a path.
    pub fn workspace(&self) -> Result<Option<PathBuf>, ContextError> {
        match self.table.get("workspace") {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(|path| Some(absolute(self.base(), Path::new(path))))
                .ok_or_else(|| {
                    refuse(
                        &self.path,
                        format!("`[context.{}] workspace` is a path", self.name),
                    )
                }),
        }
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

/// The three execution modes an invocation runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// No workspace and no vault.
    Bare,
    /// A workspace marked by `hql.toml` alone.
    Workspace,
    /// A workspace marked by `.hmd/`, which is also the vault.
    Vault,
}

impl ExecutionMode {
    /// The name `hql context` reports.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bare => "bare",
            Self::Workspace => "workspace",
            Self::Vault => "vault",
        }
    }
}

/// How the workspace of an invocation was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceOrigin {
    /// The nearest marked directory at or above the anchor.
    Discovered,
    /// A command-line option, named as it was written.
    Option(&'static str),
    /// The selected named context binds it.
    Context(String),
    /// The embedding host supplied it.
    Host,
}

impl WorkspaceOrigin {
    /// How `hql context` names the origin.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Discovered => "discovered from the anchor".to_owned(),
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

/// The root of the HQL environment an invocation runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// The workspace directory.
    pub root: PathBuf,
    /// How it was selected.
    pub origin: WorkspaceOrigin,
}

impl Workspace {
    /// Whether the root holds `.hmd/`, which makes it a vault.
    #[must_use]
    pub fn is_vault(&self) -> bool {
        self.root.join(VAULT_MARKER).is_dir()
    }

    /// The workspace configuration file, when the root holds one.
    #[must_use]
    pub fn config_file(&self) -> Option<PathBuf> {
        let file = self.root.join(CONFIG_FILE);
        file.is_file().then_some(file)
    }
}

/// Whether a directory is a workspace boundary: it holds `.hmd/` or
/// `hql.toml`.
#[must_use]
pub fn is_boundary(dir: &Path) -> bool {
    crate::vault::is_root(dir)
}

/// The nearest workspace boundary at or above `anchor`. The walk stops at
/// the first marked directory, so an outer boundary never reaches past an
/// inner one.
#[must_use]
pub fn discover(anchor: &Path) -> Option<PathBuf> {
    anchor
        .ancestors()
        .find(|dir| is_boundary(dir))
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
    /// The directory discovery starts from; the working directory without one.
    pub anchor: Option<PathBuf>,
    /// Run with no workspace whatever the filesystem holds.
    pub bare: bool,
    /// A workspace named on the command line, with the option that named it.
    pub workspace: Option<(PathBuf, &'static str)>,
    /// A workspace an embedding host supplies; discovery does not run.
    pub host_workspace: Option<PathBuf>,
    /// Module roots named on the command line, read against the working
    /// directory.
    pub modules: Vec<PathBuf>,
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
    /// Where discovery starts.
    pub anchor: PathBuf,
    /// The workspace, absent in bare mode.
    pub workspace: Option<Workspace>,
    /// Whether bare execution was requested, rather than found for want of
    /// a boundary.
    pub bare: bool,
    /// The environment it was constructed in.
    pub environment: Environment,
    /// The system and user layers.
    pub layers: Layers,
    /// The selected named context, and the layer that selected it.
    pub context: Option<Contributed<String>>,
    /// Its definition.
    pub named: Option<NamedContext>,
    /// The workspace's `hql.toml`, in workspace and vault mode.
    pub workspace_config: Option<ConfigFile>,
    /// Every module root, in layer order, each once.
    pub module_roots: Vec<Contributed<PathBuf>>,
    /// Every extension imported for each program, in layer order, each once.
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
    /// Contradictory explicit selections, and an explicit workspace that is
    /// not one.
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
            .map(NamedContext::workspace)
            .transpose()?
            .flatten();
        let explicit = match (&request.host_workspace, &request.workspace) {
            (Some(_), Some((_, option))) => {
                return Err(ContextError::Conflict(format!(
                    "the host supplies the workspace, and {option} names another"
                )));
            }
            (Some(root), None) => Some(Workspace {
                root: root.clone(),
                origin: WorkspaceOrigin::Host,
            }),
            (None, Some((root, option))) => Some(Workspace {
                root: root.clone(),
                origin: WorkspaceOrigin::Option(option),
            }),
            (None, None) => None,
        };
        if request.bare
            && let Some(explicit) = &explicit
        {
            return Err(ContextError::Conflict(format!(
                "--bare runs with no workspace, and {} names one",
                explicit.origin.label()
            )));
        }
        if let (Some(choice), Some(root)) = (&context, &bound)
            && chosen
        {
            if request.bare {
                return Err(ContextError::Conflict(format!(
                    "--bare runs with no workspace, and context `{}`, which {} selects, binds {}",
                    choice.value,
                    choice.origin.label(),
                    root.display()
                )));
            }
            if let Some(explicit) = &explicit
                && !same_directory(&explicit.root, root)
            {
                return Err(ContextError::Conflict(format!(
                    "context `{}`, which {} selects, binds the workspace {}, and {} names {}",
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
            bound.map(|root| Workspace {
                root,
                origin: WorkspaceOrigin::Context(name),
            })
        });
        let workspace = if request.bare {
            None
        } else if let Some(explicit) = explicit {
            if !explicit.root.is_dir() {
                return Err(ContextError::NotADirectory(explicit.root));
            }
            if explicit.origin != WorkspaceOrigin::Host && !is_boundary(&explicit.root) {
                return Err(ContextError::NotAWorkspace(explicit.root));
            }
            Some(explicit)
        } else {
            discover(&anchor).map(|root| Workspace {
                root,
                origin: WorkspaceOrigin::Discovered,
            })
        };
        let workspace_config = workspace
            .as_ref()
            .map(|workspace| ConfigFile::read(Origin::Workspace, workspace.root.join(CONFIG_FILE)))
            .transpose()?;
        let mut context = Self {
            cwd: environment.cwd.clone(),
            anchor,
            workspace,
            bare: request.bare,
            environment: environment.clone(),
            layers,
            context,
            named,
            workspace_config,
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

    /// Gather the module roots and extension settings of every layer, in
    /// order: the system file, the user file, the selected context, the HQL
    /// home's modules, the workspace's `hql.toml`, `$HQL_PATH`, then
    /// `--modules`.
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
        if let Some(file) = self.workspace_config.clone() {
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

    /// Hand the effective module roots and extension settings to the
    /// extension configuration a vault carries.
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

    /// Every configuration file of the layering, in order, present or not.
    pub fn config_files(&self) -> impl Iterator<Item = &ConfigFile> {
        self.layers.files().chain(self.workspace_config.iter())
    }

    /// The mode the context puts the invocation in.
    #[must_use]
    pub fn mode(&self) -> ExecutionMode {
        match &self.workspace {
            None => ExecutionMode::Bare,
            Some(workspace) if workspace.is_vault() => ExecutionMode::Vault,
            Some(_) => ExecutionMode::Workspace,
        }
    }

    /// The vault root: the workspace root when it holds `.hmd/`.
    #[must_use]
    pub fn vault(&self) -> Option<&Path> {
        self.workspace
            .as_ref()
            .filter(|workspace| workspace.is_vault())
            .map(|workspace| workspace.root.as_path())
    }

    /// The workspace root, absent in bare mode.
    #[must_use]
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.root.as_path())
    }
}

/// Whether two paths name one directory.
fn same_directory(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

/// A path as given when absolute, else joined to `base`.
fn absolute(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
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

    #[test]
    fn a_directory_with_no_boundary_above_it_runs_bare() {
        let root = tempfile::tempdir().unwrap();
        let context =
            ExecutionContext::construct(&Request::default(), &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        assert_eq!(context.workspace, None);
        assert_eq!(context.vault(), None);
        assert_eq!(context.anchor, root.path());
    }

    #[test]
    fn the_walk_starts_at_the_anchor_not_the_working_directory() {
        let root = tempfile::tempdir().unwrap();
        let vault = root.path().join("a");
        fs::create_dir_all(vault.join(".hmd")).unwrap();
        fs::create_dir_all(vault.join("b/c")).unwrap();
        let elsewhere = root.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let context =
            ExecutionContext::construct(&anchored(&vault.join("b/c")), &environment(&elsewhere))
                .unwrap();
        assert_eq!(context.mode(), ExecutionMode::Vault);
        assert_eq!(context.workspace_root(), Some(vault.as_path()));
        assert_eq!(context.vault(), Some(vault.as_path()));
        assert_eq!(context.cwd, elsewhere);
        assert_eq!(context.anchor, vault.join("b/c"));
    }

    #[test]
    fn a_relative_anchor_is_read_against_the_working_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(CONFIG_FILE), "").unwrap();
        fs::create_dir_all(root.path().join("src")).unwrap();
        let context =
            ExecutionContext::construct(&anchored(Path::new("src")), &environment(root.path()))
                .unwrap();
        assert_eq!(context.anchor, root.path().join("src"));
        assert_eq!(context.mode(), ExecutionMode::Workspace);
        assert_eq!(context.workspace_root(), Some(root.path()));
        assert_eq!(context.vault(), None);
    }

    #[test]
    fn the_nearest_boundary_wins_and_an_outer_vault_is_not_merged() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".hmd")).unwrap();
        let inner = root.path().join("inner");
        fs::create_dir_all(inner.join("src")).unwrap();
        fs::write(inner.join(CONFIG_FILE), "").unwrap();
        let context =
            ExecutionContext::construct(&anchored(&inner.join("src")), &environment(root.path()))
                .unwrap();
        assert_eq!(context.mode(), ExecutionMode::Workspace);
        assert_eq!(context.workspace_root(), Some(inner.as_path()));
        assert_eq!(context.vault(), None);
    }

    #[test]
    fn both_markers_in_one_directory_are_one_vault_workspace() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".hmd")).unwrap();
        fs::write(root.path().join(CONFIG_FILE), "").unwrap();
        let context =
            ExecutionContext::construct(&Request::default(), &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Vault);
        let workspace = context.workspace.as_ref().unwrap();
        assert_eq!(workspace.config_file(), Some(root.path().join(CONFIG_FILE)));
        assert_eq!(context.vault(), Some(root.path()));
    }

    #[test]
    fn bare_suppresses_discovery() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".hmd")).unwrap();
        let request = Request {
            bare: true,
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(root.path())).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
    }

    #[test]
    fn an_explicit_workspace_replaces_the_walk_and_keeps_the_anchor() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir_all(repo.join("src/jobs")).unwrap();
        fs::write(repo.join(CONFIG_FILE), "").unwrap();
        let nested = repo.join("src/jobs");
        fs::write(nested.join(CONFIG_FILE), "").unwrap();
        let request = Request {
            anchor: Some(nested.clone()),
            workspace: Some((repo.clone(), "--workspace")),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(root.path())).unwrap();
        assert_eq!(context.workspace_root(), Some(repo.as_path()));
        assert_eq!(context.anchor, nested);
        assert_eq!(
            context.workspace.unwrap().origin,
            WorkspaceOrigin::Option("--workspace")
        );
    }

    #[test]
    fn an_explicit_workspace_must_be_a_marked_directory() {
        let root = tempfile::tempdir().unwrap();
        let request = |path: PathBuf| Request {
            workspace: Some((path, "--workspace")),
            ..Request::default()
        };
        let missing = root.path().join("missing");
        assert_eq!(
            ExecutionContext::construct(&request(missing.clone()), &environment(root.path())),
            Err(ContextError::NotADirectory(missing))
        );
        assert_eq!(
            ExecutionContext::construct(
                &request(root.path().to_path_buf()),
                &environment(root.path())
            ),
            Err(ContextError::NotAWorkspace(root.path().to_path_buf()))
        );
    }

    #[test]
    fn bare_with_a_workspace_is_a_conflict() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(CONFIG_FILE), "").unwrap();
        let request = Request {
            bare: true,
            workspace: Some((root.path().to_path_buf(), "--workspace")),
            ..Request::default()
        };
        let refused = ExecutionContext::construct(&request, &environment(root.path())).unwrap_err();
        assert!(matches!(refused, ContextError::Conflict(_)), "{refused}");
        assert!(refused.to_string().contains("--bare"), "{refused}");
    }

    #[test]
    fn a_host_workspace_is_authoritative_and_needs_no_marker() {
        let root = tempfile::tempdir().unwrap();
        let outer = root.path();
        fs::create_dir_all(outer.join(".hmd")).unwrap();
        let hosted = outer.join("hosted");
        fs::create_dir_all(&hosted).unwrap();
        let request = Request {
            host_workspace: Some(hosted.clone()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &environment(outer)).unwrap();
        assert_eq!(context.workspace_root(), Some(hosted.as_path()));
        assert_eq!(context.mode(), ExecutionMode::Workspace);
        assert_eq!(context.workspace.unwrap().origin, WorkspaceOrigin::Host);
        let both = Request {
            host_workspace: Some(hosted.clone()),
            workspace: Some((hosted, "--workspace")),
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

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
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
        fs::create_dir_all(root.join("tree/.hmd")).unwrap();
        write(
            &root.join("etc/config.toml"),
            "[modules]\nroots = [\"lib\"]\n[extensions]\nimport = [\"search\"]\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[modules]\nroots = [\"mine\"]\n[extensions]\nprelude = false\nimport = [\"search\", \"net\"]\n",
        );
        fs::create_dir_all(root.join("home/.hql/modules")).unwrap();
        let mut environment = configured(root);
        environment.hql_path =
            Some(std::env::join_paths([root.join("path/a"), root.join("path/b")]).unwrap());
        let request = Request {
            bare: true,
            modules: vec![PathBuf::from("cli")],
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
            vec![("search", Origin::System), ("net", Origin::User)]
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
        // `~/.hql` is never the vault.
        assert_eq!(context.vault(), None);
    }

    #[test]
    fn a_workspace_adds_its_own_roots_and_its_relative_paths_read_against_it() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("tree/hql.toml"),
            "[modules]\nroots = [\"./lib\"]\n[extensions]\nprelude = true\n",
        );
        write(
            &root.join("home/.hql/config.toml"),
            "[extensions]\nprelude = false\n",
        );
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            roots(&context),
            vec![(root.join("tree/lib"), Origin::Workspace)]
        );
        assert_eq!(context.prelude.origin, Origin::Workspace);
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
                (Origin::Workspace, true)
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

    const CONTEXTS: &str = "\
current-context = \"work\"

[context.work.modules]
roots = [\"work-lib\"]

[context.experimental.extensions]
prelude = false

[context.production]
workspace = \"../srv/production\"
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
            "[context.default.extensions]\nimport = [\"search\"]\n",
        );
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            selected(&context),
            Some((DEFAULT_CONTEXT.to_owned(), Origin::Default))
        );
        assert_eq!(context.imports[0].value, "search");
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
    fn a_context_that_binds_a_workspace_replaces_discovery() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        let production = root.join("home/srv/production");
        fs::create_dir_all(production.join(".hmd")).unwrap();
        fs::create_dir_all(root.join("tree/.hmd")).unwrap();
        let request = Request {
            context: Some("production".to_owned()),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&request, &configured(root)).unwrap();
        let workspace = context.workspace.as_ref().unwrap();
        assert!(same_directory(&workspace.root, &production));
        assert_eq!(
            workspace.origin,
            WorkspaceOrigin::Context("production".to_owned())
        );
        assert_eq!(context.mode(), ExecutionMode::Vault);

        // The same context reused without a binding leaves discovery alone.
        let context = ExecutionContext::construct(&Request::default(), &configured(root)).unwrap();
        assert_eq!(
            context.workspace.unwrap().origin,
            WorkspaceOrigin::Discovered
        );
    }

    #[test]
    fn contradicting_a_selected_context_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(&root.join("home/.hql/config.toml"), CONTEXTS);
        fs::create_dir_all(root.join("home/srv/production/.hmd")).unwrap();
        fs::create_dir_all(root.join("tree/.hmd")).unwrap();
        let production = || Some("production".to_owned());
        for request in [
            Request {
                context: production(),
                bare: true,
                ..Request::default()
            },
            Request {
                context: production(),
                workspace: Some((root.join("tree"), "--workspace")),
                ..Request::default()
            },
        ] {
            let refused = ExecutionContext::construct(&request, &configured(root)).unwrap_err();
            assert!(matches!(refused, ContextError::Conflict(_)), "{refused}");
            assert!(refused.to_string().contains("production"), "{refused}");
        }
        // Naming the workspace the context binds is no contradiction.
        let agreeing = Request {
            context: production(),
            workspace: Some((root.join("home/srv/production"), "--workspace")),
            ..Request::default()
        };
        assert!(ExecutionContext::construct(&agreeing, &configured(root)).is_ok());
    }

    #[test]
    fn a_persistent_context_yields_to_an_explicit_workspace_or_bare() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        write(
            &root.join("home/.hql/config.toml"),
            &CONTEXTS.replace(
                "current-context = \"work\"",
                "current-context = \"production\"",
            ),
        );
        fs::create_dir_all(root.join("home/srv/production/.hmd")).unwrap();
        fs::create_dir_all(root.join("tree/.hmd")).unwrap();
        let bare = Request {
            bare: true,
            ..Request::default()
        };
        let context = ExecutionContext::construct(&bare, &configured(root)).unwrap();
        assert_eq!(context.mode(), ExecutionMode::Bare);
        let named = Request {
            workspace: Some((root.join("tree"), "--workspace")),
            ..Request::default()
        };
        let context = ExecutionContext::construct(&named, &configured(root)).unwrap();
        assert_eq!(context.workspace_root(), Some(root.join("tree").as_path()));
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
            "current-context = \"a\"\n[context.a]\nworkspace = \"/x\"\n",
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
}
