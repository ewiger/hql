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

    fn error(&self, message: impl Into<String>) -> ContextError {
        ContextError::Config {
            path: self.path.clone(),
            message: message.into(),
        }
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
    /// `file` names the file in a refusal.
    fn of(table: &toml::Table, base: &Path, file: &ConfigFile) -> Result<Self, ContextError> {
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
                .ok_or_else(|| file.error(format!("`[{section}] {key}` is a list of strings")))
        };
        let prelude = match table
            .get("extensions")
            .and_then(|section| section.get("prelude"))
        {
            None => None,
            Some(value) => Some(
                value
                    .as_bool()
                    .ok_or_else(|| file.error("`[extensions] prelude` is true or false"))?,
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
    /// order: the system file, the user file, the HQL home's modules, the
    /// workspace's `hql.toml`, `$HQL_PATH`, then `--modules`.
    fn contribute(&mut self, request: &Request) -> Result<(), ContextError> {
        let layers: Vec<ConfigFile> = self.layers.files().cloned().collect();
        for file in &layers {
            self.take(file)?;
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
        let settings = Settings::of(table, file.base(), file)?;
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
        let canonical =
            |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let identity = canonical(&path);
        if self
            .module_roots
            .iter()
            .any(|held| canonical(&held.value) == identity)
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
}
