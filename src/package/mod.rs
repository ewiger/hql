//! Packages: extensions installed beside the binary rather than compiled
//! into it (HQL-0057).
//!
//! An installed package is a directory holding `package.toml`, the module
//! root with its contracts, and the library carrying its extension. A
//! process activates its packages once, before the first program is
//! checked: each is loaded, its modules join the module graph after the
//! builtin ones, its extension joins the extension set, and the types its
//! modules declare join the type lattice every program starts from. A
//! package that cannot be loaded, or whose contracts and extension do not
//! agree, is refused whole and leaves the rest as they were.

pub mod build;
mod load;
pub mod manifest;

use crate::extensions::Extension;
use crate::extensions::binding::{Cache, ExtensionSet, Graph};
use manifest::Installed;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The directory of installed packages under the HQL home.
pub const LIBRARY_DIRECTORY: &str = "lib";

/// A package the process loaded.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// Its record.
    pub installed: Installed,
    /// Its directory.
    pub directory: PathBuf,
}

/// A package the process found and did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// Its name.
    pub name: String,
    /// The modules it would have provided.
    pub provides: Vec<String>,
    /// Why it was refused.
    pub reason: String,
}

/// The packages of the process.
#[derive(Debug)]
pub struct Active {
    /// Every module the process binds: the builtin ones, then each loaded
    /// package's.
    pub(crate) graph: Graph,
    /// Every extension the process binds, in the same order.
    pub(crate) extensions: ExtensionSet,
    /// The packages loaded, in loading order.
    pub loaded: Vec<Loaded>,
    /// The packages found and refused.
    pub refused: Vec<Refused>,
}

static ACTIVE: OnceLock<Active> = OnceLock::new();

thread_local! {
    /// The graph and extension set under trial while a package is being
    /// admitted: what binding sees until activation is over.
    static TRIAL: Cell<Option<(Graph, ExtensionSet)>> = const { Cell::new(None) };
}

/// The modules the process binds: the builtin ones until packages are
/// activated, and with each loaded package's after.
pub(crate) fn graph() -> Graph {
    if let Some((graph, _)) = TRIAL.get() {
        return graph;
    }
    ACTIVE
        .get()
        .map_or(crate::library::MODULES, |active| active.graph)
}

/// The extensions the process binds.
pub(crate) fn extensions() -> ExtensionSet {
    if let Some((_, extensions)) = TRIAL.get() {
        return extensions;
    }
    ACTIVE
        .get()
        .map_or(crate::extensions::REGISTERED, |active| active.extensions)
}

/// The packages of the process, once activated.
#[must_use]
pub fn active() -> Option<&'static Active> {
    ACTIVE.get()
}

/// The refusal of the package that would have provided a module.
#[must_use]
pub fn refusal(module: &str) -> Option<&'static Refused> {
    active()?
        .refused
        .iter()
        .find(|refused| refused.provides.iter().any(|held| held == module))
}

/// The directories of the packages installed under an HQL home, by name.
#[must_use]
pub fn installed(home: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(home.join(LIBRARY_DIRECTORY))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(manifest::INSTALLED).is_file())
        .collect();
    found.sort();
    found
}

/// The directory a package is installed in under an HQL home.
#[must_use]
pub fn directory(home: &Path, package: &str) -> PathBuf {
    home.join(LIBRARY_DIRECTORY).join(package)
}

/// One package under admission: what it adds to the graph and the set.
struct Candidate {
    loaded: Loaded,
    modules: Vec<(&'static str, &'static str)>,
    extension: &'static Extension,
}

/// Read and open an installed package, without binding it.
fn open(directory: &Path) -> Result<Candidate, Refused> {
    let name = directory.file_name().map_or_else(
        || directory.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let refuse = |provides: &[String], reason: String| Refused {
        name: name.clone(),
        provides: provides.to_vec(),
        reason,
    };
    let installed = Installed::read(directory).map_err(|reason| refuse(&[], reason))?;
    let refuse = |reason: String| Refused {
        name: installed.name.clone(),
        provides: installed.provides.clone(),
        reason,
    };
    if let Some(stale) = installed.stale() {
        return Err(refuse(stale));
    }
    let root = directory.join("modules");
    let mut modules = Vec::new();
    for module in &installed.provides {
        let file = manifest::module_file(&root, module).ok_or_else(|| {
            refuse(format!(
                "package `{}` provides `{module}`, which has no file under {}",
                installed.name,
                root.display()
            ))
        })?;
        let source = std::fs::read_to_string(&file)
            .map_err(|error| refuse(format!("{}: {error}", file.display())))?;
        // Held for the process, as the builtin modules' sources are.
        let name: &'static str = Box::leak(module.clone().into_boxed_str());
        let source: &'static str = Box::leak(source.into_boxed_str());
        modules.push((name, source));
    }
    let extension =
        load::open(&installed.name, &directory.join(&installed.library)).map_err(&refuse)?;
    if extension.id() != installed.name {
        return Err(refuse(format!(
            "the library of package `{}` carries the extension `{}`",
            installed.name,
            extension.id()
        )));
    }
    let stated: Vec<&str> = extension.modules().collect();
    if stated != installed.provides {
        return Err(refuse(format!(
            "package `{}` provides {} by its record and {} by its extension",
            installed.name,
            list(&installed.provides),
            list(&stated)
        )));
    }
    Ok(Candidate {
        loaded: Loaded {
            installed,
            directory: directory.to_path_buf(),
        },
        modules,
        extension,
    })
}

/// Names as a diagnostic lists them.
fn list<T: AsRef<str>>(names: &[T]) -> String {
    names
        .iter()
        .map(|name| format!("`{}`", name.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Enter the types a package's modules declare into the lattice in force.
fn declare_types(modules: &[(&'static str, &'static str)]) -> Result<(), String> {
    for (name, source) in modules {
        let program = crate::parser::parse(source)
            .map_err(|error| format!("module `{name}`: {}", error.message()))?;
        for statement in &program.statements {
            if let crate::ast::Stmt::Type(declaration) = statement {
                crate::declarations::enter_lattice(declaration)
                    .map_err(|error| format!("module `{name}`: {}", error.message()))?;
            }
        }
    }
    Ok(())
}

/// Activate the packages of the process: those in `directories`, in order,
/// each an installed package's directory. The first activation is the
/// process's; a later call returns it unchanged.
pub fn activate(directories: &[PathBuf]) -> &'static Active {
    ACTIVE.get_or_init(|| admit(directories))
}

fn admit(directories: &[PathBuf]) -> Active {
    let mut graph: Graph = crate::library::MODULES;
    let mut extensions: ExtensionSet = crate::extensions::REGISTERED;
    let mut loaded: Vec<Loaded> = Vec::new();
    let mut refused: Vec<Refused> = Vec::new();
    // The lattice packages' types are entered into: the built-in one, held
    // on this thread until every package is admitted or refused.
    let Ok(builtin) = crate::types::program::current() else {
        return Active {
            graph,
            extensions,
            loaded,
            refused,
        };
    };
    crate::types::program::install(builtin);
    // What each directory states it provides, before any library is
    // opened. The same package found twice is one package, the first
    // found; a module two packages provide has no provider, and neither
    // package is loaded: the order of discovery picks no winner.
    let mut stated: Vec<(String, Vec<String>)> = Vec::new();
    for directory in directories {
        if let Ok(installed) = Installed::read(directory)
            && !stated.iter().any(|(name, _)| *name == installed.name)
        {
            stated.push((installed.name, installed.provides));
        }
    }
    let rival = |package: &str, provides: &[String]| {
        stated.iter().find_map(|(other, held)| {
            (other != package)
                .then(|| provides.iter().find(|module| held.contains(module)))
                .flatten()
                .map(|module| (other.clone(), module.clone()))
        })
    };
    for directory in directories {
        if let Ok(installed) = Installed::read(directory)
            && !loaded
                .iter()
                .any(|held| held.installed.name == installed.name)
            && !refused.iter().any(|held| held.name == installed.name)
            && let Some((other, module)) = rival(&installed.name, &installed.provides)
        {
            refused.push(Refused {
                reason: format!(
                    "packages `{}` and `{other}` both provide `{module}`: a module has one \
                     provider, so neither is loaded",
                    installed.name
                ),
                name: installed.name,
                provides: installed.provides,
            });
            continue;
        }
        let candidate = match open(directory) {
            Ok(candidate) => candidate,
            Err(refusal) => {
                refused.push(refusal);
                continue;
            }
        };
        let installed = &candidate.loaded.installed;
        let refuse = |reason: String| Refused {
            name: installed.name.clone(),
            provides: installed.provides.clone(),
            reason,
        };
        if loaded
            .iter()
            .any(|held| held.installed.name == installed.name)
            || refused.iter().any(|held| held.name == installed.name)
        {
            // The same package found twice: the first found is the one.
            continue;
        }
        if let Some((module, _)) = candidate
            .modules
            .iter()
            .find(|(module, _)| graph.iter().any(|(held, _)| held == module))
        {
            let owner = extensions
                .iter()
                .find(|extension| extension.modules().any(|held| held == *module))
                .map_or("std", |extension| extension.id());
            refused.push(refuse(format!(
                "package `{}` provides `{module}`, which `{owner}` provides: a module has one \
                 provider",
                installed.name
            )));
            continue;
        }
        let before = crate::types::program::current().ok();
        let mut trial_graph = graph.to_vec();
        trial_graph.extend_from_slice(&candidate.modules);
        let mut trial_extensions = extensions.to_vec();
        trial_extensions.push(candidate.extension);
        let trial_graph: Graph = Box::leak(trial_graph.into_boxed_slice());
        let trial_extensions: ExtensionSet = Box::leak(trial_extensions.into_boxed_slice());
        TRIAL.set(Some((trial_graph, trial_extensions)));
        let admitted = declare_types(&candidate.modules).and_then(|()| {
            Cache::process()
                .frozen(trial_graph, trial_extensions)
                .map(|_| ())
        });
        match admitted {
            Ok(()) => {
                graph = trial_graph;
                extensions = trial_extensions;
                loaded.push(candidate.loaded);
            }
            Err(reason) => {
                // The lattice and the graph are as they were before it.
                if let Some(before) = before {
                    crate::types::program::install(before);
                }
                TRIAL.set(Some((graph, extensions)));
                refused.push(refuse(format!(
                    "package `{}` is refused: {reason}",
                    installed.name
                )));
            }
        }
    }
    TRIAL.set(None);
    if !loaded.is_empty()
        && let Ok(extended) = crate::types::program::current()
    {
        crate::types::program::extend_base(extended);
    }
    crate::types::program::reset();
    Active {
        graph,
        extensions,
        loaded,
        refused,
    }
}

/// Activate the packages a workspace and an HQL home name: the crates
/// `[packages] paths` lists, each built where its build is missing or old,
/// then the packages installed under the home.
pub fn activate_configured(paths: &[PathBuf], home: Option<&Path>) -> &'static Active {
    if let Some(active) = ACTIVE.get() {
        return active;
    }
    let mut directories = Vec::new();
    let mut unbuilt = Vec::new();
    for path in paths {
        match build::local(path) {
            Ok(directory) => directories.push(directory),
            Err(reason) => unbuilt.push(Refused {
                name: manifest::Crate::read(path)
                    .map_or_else(|_| path.display().to_string(), |held| held.package),
                provides: manifest::Crate::read(path)
                    .map(|held| held.provides)
                    .unwrap_or_default(),
                reason,
            }),
        }
    }
    if let Some(home) = home {
        directories.extend(installed(home));
    }
    ACTIVE.get_or_init(|| {
        let mut active = admit(&directories);
        active.refused.extend(unbuilt);
        active
    })
}
