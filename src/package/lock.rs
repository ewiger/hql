//! A level's record of its packages (HQL-0059): `config.toml` declares the
//! packages a level depends on by name, and `lock.toml` beside it pins each
//! to one version and source.
//!
//! A level is a directory holding `config.toml`, `packages/`, `cache/` and
//! `lock.toml`: a project's `.hql/`, or the HQL home. `hql package` writes
//! both files of the level it acts on; a run reads the declarations and
//! loads what `packages/` holds.

use super::manifest::Source;
use std::path::{Path, PathBuf};

/// The lock file of a level.
pub const FILE: &str = "lock.toml";

/// The configuration file of a level.
pub const CONFIG: &str = "config.toml";

/// One package a level pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pinned {
    /// The package's name.
    pub name: String,
    /// The version installed.
    pub version: String,
    /// Where it came from.
    pub source: Source,
}

/// The packages a level pins, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lock {
    /// Each pinned package, in name order.
    pub packages: Vec<Pinned>,
}

impl Lock {
    /// Read a level's `lock.toml`; an absent one pins nothing.
    ///
    /// # Errors
    ///
    /// The file exists and is not a lock file, naming it.
    pub fn read(level: &Path) -> Result<Self, String> {
        let path = level.join(FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let refuse = |message: &str| format!("{}: {message}", path.display());
        let table: toml::Table = toml::from_str(&text).map_err(|error| refuse(error.message()))?;
        let mut packages = Vec::new();
        for entry in table
            .get("package")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let field = |key: &str| {
                entry
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| refuse(&format!("a `[[package]]` names its `{key}`")))
            };
            let source = match field("source")?.as_str() {
                "path" => Source::Path(PathBuf::from(field("path")?)),
                "registry" => Source::Registry {
                    registry: field("registry")?,
                    tag: field("tag")?,
                },
                other => {
                    return Err(refuse(&format!(
                        "a package's `source` is `path` or `registry`, not `{other}`"
                    )));
                }
            };
            packages.push(Pinned {
                name: field("name")?,
                version: field("version")?,
                source,
            });
        }
        packages.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(Self { packages })
    }

    /// Write the lock file of a level, creating the level.
    ///
    /// # Errors
    ///
    /// The file cannot be written, naming it.
    pub fn write(&self, level: &Path) -> Result<(), String> {
        let mut text = String::from(
            "# Written by `hql package`: each package this level depends on, pinned to\n\
             # one version and source. Commit it; `hql package install` reinstalls it.\n",
        );
        for pinned in &self.packages {
            let quoted = |value: &str| toml::Value::String(value.to_owned()).to_string();
            text.push_str("\n[[package]]\n");
            text.push_str(&format!("name = {}\n", quoted(&pinned.name)));
            text.push_str(&format!("version = {}\n", quoted(&pinned.version)));
            match &pinned.source {
                Source::Path(path) => {
                    text.push_str("source = \"path\"\n");
                    text.push_str(&format!("path = {}\n", quoted(&path.display().to_string())));
                }
                Source::Registry { registry, tag } => {
                    text.push_str("source = \"registry\"\n");
                    text.push_str(&format!("registry = {}\n", quoted(registry)));
                    text.push_str(&format!("tag = {}\n", quoted(tag)));
                }
            }
        }
        std::fs::create_dir_all(level).map_err(|error| format!("{}: {error}", level.display()))?;
        let path = level.join(FILE);
        std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// The pin of a package, if the level has one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Pinned> {
        self.packages.iter().find(|pinned| pinned.name == name)
    }

    /// Pin a package, replacing its earlier pin.
    pub fn pin(&mut self, pinned: Pinned) {
        self.packages.retain(|held| held.name != pinned.name);
        self.packages.push(pinned);
        self.packages
            .sort_by(|left, right| left.name.cmp(&right.name));
    }

    /// Drop a package's pin.
    pub fn unpin(&mut self, name: &str) {
        self.packages.retain(|held| held.name != name);
    }
}

/// The packages a level's `config.toml` declares in `[packages]
/// dependencies`, in the order written.
///
/// # Errors
///
/// The file is not TOML, or `dependencies` is not a list of names.
pub fn dependencies(level: &Path) -> Result<Vec<String>, String> {
    let path = level.join(CONFIG);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let table: toml::Table = toml::from_str(&text)
        .map_err(|error| format!("{}: {}", path.display(), error.message()))?;
    let Some(value) = table
        .get("packages")
        .and_then(|packages| packages.get("dependencies"))
    else {
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
        .ok_or_else(|| {
            format!(
                "{}: `[packages] dependencies` is a list of package names, with no version",
                path.display()
            )
        })
}

/// Declare a package in a level's `config.toml`, or withdraw it, keeping
/// every other line of the file as it was.
///
/// # Errors
///
/// The file cannot be read or written, or does not parse.
pub fn declare(level: &Path, name: &str, declared: bool) -> Result<(), String> {
    let path = level.join(CONFIG);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let mut names = dependencies(level)?;
    let held = names.iter().any(|held| held == name);
    if held == declared {
        return Ok(());
    }
    if declared {
        names.push(name.to_owned());
    } else {
        names.retain(|held| held != name);
    }
    let written = with_dependencies(&text, &names);
    std::fs::create_dir_all(level).map_err(|error| format!("{}: {error}", level.display()))?;
    std::fs::write(&path, written).map_err(|error| format!("{}: {error}", path.display()))
}

/// A configuration file's text with `[packages] dependencies` set to
/// `names`: the key is replaced where it is written, added under
/// `[packages]` where the table has none, and the table added at the end
/// where the file has none.
#[must_use]
pub fn with_dependencies(text: &str, names: &[String]) -> String {
    let list: Vec<String> = names
        .iter()
        .map(|name| toml::Value::String(name.clone()).to_string())
        .collect();
    let assignment = format!("dependencies = [{}]", list.join(", "));
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let header = lines.iter().position(|line| line.trim() == "[packages]");
    let Some(header) = header else {
        let mut written = text.trim_end().to_owned();
        if !written.is_empty() {
            written.push_str("\n\n");
        }
        written.push_str(&format!("[packages]\n{assignment}\n"));
        return written;
    };
    let end = lines[header + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map_or(lines.len(), |offset| header + 1 + offset);
    let start = (header + 1..end).find(|&index| {
        lines[index]
            .trim_start()
            .strip_prefix("dependencies")
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    });
    match start {
        Some(start) => {
            // The value may span lines: it ends where its brackets balance.
            let mut depth = 0_i32;
            let mut last = start;
            for (index, line) in lines.iter().enumerate().take(end).skip(start) {
                depth += line.matches('[').count() as i32 - line.matches(']').count() as i32;
                last = index;
                if depth <= 0 {
                    break;
                }
            }
            lines.splice(start..=last, [assignment]);
        }
        None => lines.insert(header + 1, assignment),
    }
    let mut written = lines.join("\n");
    written.push('\n');
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dependency_is_added_replaced_and_dropped_keeping_every_other_line() {
        let names = |list: &[&str]| {
            list.iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            with_dependencies("", &names(&["ml"])),
            "[packages]\ndependencies = [\"ml\"]\n"
        );
        assert_eq!(
            with_dependencies("# mine\n[modules]\nroots = [\"lib\"]\n", &names(&["ml"])),
            "# mine\n[modules]\nroots = [\"lib\"]\n\n[packages]\ndependencies = [\"ml\"]\n"
        );
        assert_eq!(
            with_dependencies(
                "[packages]\npaths = [\"../x\"]\n[modules]\n",
                &names(&["ml", "geo"])
            ),
            "[packages]\ndependencies = [\"ml\", \"geo\"]\npaths = [\"../x\"]\n[modules]\n"
        );
        assert_eq!(
            with_dependencies(
                "[packages]\ndependencies = [\n  \"ml\",\n  \"geo\",\n]\npaths = []\n",
                &names(&["geo"])
            ),
            "[packages]\ndependencies = [\"geo\"]\npaths = []\n"
        );
    }

    #[test]
    fn a_lock_is_written_and_read_back_in_name_order() {
        let level = tempfile::tempdir().unwrap();
        let mut lock = Lock::default();
        lock.pin(Pinned {
            name: "ml".to_owned(),
            version: "0.1.0".to_owned(),
            source: Source::Registry {
                registry: "https://example.org/hql-packages.yaml".to_owned(),
                tag: "hql-ext-ml-v0.1.0".to_owned(),
            },
        });
        lock.pin(Pinned {
            name: "geo".to_owned(),
            version: "0.2.0".to_owned(),
            source: Source::Path(PathBuf::from("/code/hql-ext-geo")),
        });
        lock.write(level.path()).unwrap();
        let read = Lock::read(level.path()).unwrap();
        assert_eq!(read, lock);
        assert_eq!(read.packages[0].name, "geo");
        lock.unpin("geo");
        assert!(lock.get("geo").is_none());
    }
}
