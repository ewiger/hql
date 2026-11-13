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

use crate::reporting::CONFIG_FILE;
use std::path::{Path, PathBuf};

/// The directory that marks a HyperMarkdown vault.
pub const VAULT_MARKER: &str = ".hmd";

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// The process working directory.
    pub cwd: PathBuf,
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
        Ok(Self { cwd })
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl ExecutionContext {
    /// Construct the context of a request in an environment.
    ///
    /// # Errors
    ///
    /// Contradictory explicit selections, and an explicit workspace that is
    /// not one.
    pub fn construct(request: &Request, environment: &Environment) -> Result<Self, ContextError> {
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
        Ok(Self {
            cwd: environment.cwd.clone(),
            anchor,
            workspace,
            bare: request.bare,
        })
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
}
