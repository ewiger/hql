//! What a package states about itself: in its crate's manifest before it
//! is built, and in `package.toml` once it is installed.

use std::path::{Path, PathBuf};

/// The file of an installed package.
pub const INSTALLED: &str = "package.toml";

/// Where an installed package's source came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A crate on this machine: `hql package install --path`, or a crate
    /// `[packages] paths` lists.
    Path(PathBuf),
    /// An entry of a registry, at a tag of its repository.
    Registry {
        /// The registry's address.
        registry: String,
        /// The tag the crate was checked out at.
        tag: String,
    },
}

/// A package's crate, read from its `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crate {
    /// The crate's directory.
    pub dir: PathBuf,
    /// The crate's name, `hql-ext-ml`.
    pub name: String,
    /// The crate's version.
    pub version: String,
    /// The package's name, `ml`.
    pub package: String,
    /// The modules the package provides, in loading order.
    pub provides: Vec<String>,
    /// The module root, resolved against the crate's directory.
    pub modules: PathBuf,
    /// The `hql` versions the crate accepts, as its dependency writes them.
    pub requirement: Option<String>,
}

/// A table of a TOML document, by the path of keys to it.
fn section<'a>(table: &'a toml::Table, path: &[&str]) -> Option<&'a toml::Table> {
    path.iter()
        .try_fold(table, |held, key| held.get(*key)?.as_table())
}

/// A list of strings under a key.
fn strings(table: &toml::Table, key: &str) -> Option<Vec<String>> {
    table
        .get(key)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// A string under a key.
fn string(table: &toml::Table, key: &str) -> Option<String> {
    table.get(key)?.as_str().map(str::to_owned)
}

impl Crate {
    /// Read the crate at a directory.
    ///
    /// # Errors
    ///
    /// Returns why the directory holds no package: no manifest, or a
    /// manifest without `[package.metadata.hql]` or one of its keys.
    pub fn read(dir: &Path) -> Result<Self, String> {
        let path = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let manifest: toml::Table =
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let missing = |what: &str| format!("{}: {what}", path.display());
        let package = section(&manifest, &["package"]).ok_or_else(|| missing("no `[package]`"))?;
        let metadata = section(&manifest, &["package", "metadata", "hql"]).ok_or_else(|| {
            missing("no `[package.metadata.hql]`: a package's crate names its package there")
        })?;
        let name = string(package, "name").ok_or_else(|| missing("`[package]` has no `name`"))?;
        // A version inherited from a workspace is not written here; the
        // build reports the one `cargo` resolves.
        let version = string(package, "version").unwrap_or_default();
        let requirement = manifest
            .get("dependencies")
            .and_then(|dependencies| dependencies.get("hql"))
            .and_then(|written| match written {
                toml::Value::String(requirement) => Some(requirement.clone()),
                toml::Value::Table(table) => string(table, "version"),
                _ => None,
            });
        Ok(Self {
            dir: dir.to_path_buf(),
            name,
            version,
            package: string(metadata, "package")
                .ok_or_else(|| missing("`[package.metadata.hql]` has no `package`"))?,
            provides: strings(metadata, "provides").ok_or_else(|| {
                missing("`[package.metadata.hql] provides` is a list of module names")
            })?,
            modules: dir.join(
                string(metadata, "modules")
                    .ok_or_else(|| missing("`[package.metadata.hql]` has no `modules`"))?,
            ),
            requirement,
        })
    }

    /// The file name of the crate's library on this platform.
    #[must_use]
    pub fn library(&self) -> String {
        format!(
            "{}{}{}",
            std::env::consts::DLL_PREFIX,
            self.name.replace('-', "_"),
            std::env::consts::DLL_SUFFIX
        )
    }
}

/// The file of a module under a module root: `ml/metrics.hql` or
/// `ml/metrics/mod.hql` for `ml.metrics`.
#[must_use]
pub fn module_file(root: &Path, module: &str) -> Option<PathBuf> {
    let relative = module.replace('.', "/");
    [
        root.join(format!("{relative}.hql")),
        root.join(&relative).join("mod.hql"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// An installed package, as its `package.toml` records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The package's name.
    pub name: String,
    /// The crate it was built from.
    pub krate: String,
    /// The crate's version.
    pub version: String,
    /// The modules it provides, in loading order.
    pub provides: Vec<String>,
    /// Where the source came from.
    pub source: Source,
    /// The `hql` version it was built against.
    pub hql: String,
    /// The compiler it was built with.
    pub rustc: String,
    /// The target it was built for.
    pub target: String,
    /// The library's file name, beside `package.toml`.
    pub library: String,
    /// When it was built.
    pub built: String,
}

impl Installed {
    /// Read the record of an installed package.
    ///
    /// # Errors
    ///
    /// Returns why the directory holds no readable record.
    pub fn read(dir: &Path) -> Result<Self, String> {
        let path = dir.join(INSTALLED);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let record: toml::Table =
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let missing = |what: &str| format!("{}: no `{what}`", path.display());
        let field = |table: &str, key: &str| {
            section(&record, &[table])
                .and_then(|held| string(held, key))
                .ok_or_else(|| missing(&format!("[{table}] {key}")))
        };
        let source = match field("source", "kind")?.as_str() {
            "path" => Source::Path(PathBuf::from(field("source", "path")?)),
            _ => Source::Registry {
                registry: field("source", "registry")?,
                tag: field("source", "tag")?,
            },
        };
        Ok(Self {
            name: field("package", "name")?,
            krate: field("package", "crate")?,
            version: field("package", "version")?,
            provides: section(&record, &["package"])
                .and_then(|held| strings(held, "provides"))
                .ok_or_else(|| missing("[package] provides"))?,
            source,
            hql: field("build", "hql")?,
            rustc: field("build", "rustc")?,
            target: field("build", "target")?,
            library: field("build", "library")?,
            built: field("build", "built")?,
        })
    }

    /// Write the record into the package's directory.
    ///
    /// # Errors
    ///
    /// Returns the failure writing the file met.
    pub fn write(&self, dir: &Path) -> Result<(), String> {
        let text = |value: &str| toml::Value::String(value.to_owned());
        let mut package = toml::Table::new();
        package.insert("name".to_owned(), text(&self.name));
        package.insert("crate".to_owned(), text(&self.krate));
        package.insert("version".to_owned(), text(&self.version));
        package.insert(
            "provides".to_owned(),
            toml::Value::Array(self.provides.iter().map(|module| text(module)).collect()),
        );
        let mut source = toml::Table::new();
        match &self.source {
            Source::Path(path) => {
                source.insert("kind".to_owned(), text("path"));
                source.insert("path".to_owned(), text(&path.display().to_string()));
            }
            Source::Registry { registry, tag } => {
                source.insert("kind".to_owned(), text("registry"));
                source.insert("registry".to_owned(), text(registry));
                source.insert("tag".to_owned(), text(tag));
            }
        }
        let mut build = toml::Table::new();
        build.insert("hql".to_owned(), text(&self.hql));
        build.insert("rustc".to_owned(), text(&self.rustc));
        build.insert("target".to_owned(), text(&self.target));
        build.insert("library".to_owned(), text(&self.library));
        build.insert("built".to_owned(), text(&self.built));
        let mut record = toml::Table::new();
        record.insert("package".to_owned(), toml::Value::Table(package));
        record.insert("source".to_owned(), toml::Value::Table(source));
        record.insert("build".to_owned(), toml::Value::Table(build));
        let path = dir.join(INSTALLED);
        std::fs::write(&path, record.to_string())
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Why this build cannot be loaded by the running `hql`, where it
    /// cannot: it was built against another version or with another
    /// compiler.
    #[must_use]
    pub fn stale(&self) -> Option<String> {
        (self.hql != crate::ext::HQL || self.rustc != crate::ext::RUSTC).then(|| {
            format!(
                "package `{}` was built against hql {} with rustc {}; this is hql {} with \
                 rustc {}: run `hql package rebuild {}`",
                self.name,
                self.hql,
                self.rustc,
                crate::ext::HQL,
                crate::ext::RUSTC,
                self.name
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed() -> Installed {
        Installed {
            name: "ml".to_owned(),
            krate: "hql-ext-ml".to_owned(),
            version: "0.1.0".to_owned(),
            provides: vec!["ml".to_owned(), "ml.metrics".to_owned()],
            source: Source::Path(PathBuf::from("/work/hql-ext-ml")),
            hql: crate::ext::HQL.to_owned(),
            rustc: crate::ext::RUSTC.to_owned(),
            target: crate::ext::TARGET.to_owned(),
            library: "libhql_ext_ml.dylib".to_owned(),
            built: "2026-11-14T10:00:00Z".to_owned(),
        }
    }

    #[test]
    fn a_record_reads_back_as_written() {
        let dir = tempfile::tempdir().unwrap();
        let written = installed();
        written.write(dir.path()).unwrap();
        assert_eq!(Installed::read(dir.path()), Ok(written.clone()));
        let registry = Installed {
            source: Source::Registry {
                registry: "https://example.org/hql-packages.yaml".to_owned(),
                tag: "hql-ext-ml-v0.1.0".to_owned(),
            },
            ..written
        };
        registry.write(dir.path()).unwrap();
        assert_eq!(Installed::read(dir.path()), Ok(registry));
    }

    #[test]
    fn a_build_against_another_hql_or_compiler_is_stale() {
        assert_eq!(installed().stale(), None);
        let other = Installed {
            hql: "0.0.1".to_owned(),
            ..installed()
        };
        let said = other.stale().unwrap();
        assert!(
            said.starts_with("package `ml` was built against hql 0.0.1 with rustc "),
            "{said}"
        );
        assert!(said.ends_with("run `hql package rebuild ml`"), "{said}");
        let compiler = Installed {
            rustc: "1.0.0 (old)".to_owned(),
            ..installed()
        };
        assert!(compiler.stale().is_some());
    }

    #[test]
    fn a_crate_states_its_package_in_its_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"hql-ext-ml\"\nversion = \"0.1.0\"\n\n[dependencies]\nhql = \
             { version = \"0.7\" }\n\n[package.metadata.hql]\npackage = \"ml\"\nprovides = \
             [\"ml\", \"ml.metrics\"]\nmodules = \"modules\"\n",
        )
        .unwrap();
        let read = Crate::read(dir.path()).unwrap();
        assert_eq!(read.package, "ml");
        assert_eq!(read.provides, ["ml", "ml.metrics"]);
        assert_eq!(read.requirement.as_deref(), Some("0.7"));
        assert_eq!(read.modules, dir.path().join("modules"));
        assert!(read.library().contains("hql_ext_ml"));
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"plain\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let refused = Crate::read(dir.path()).unwrap_err();
        assert!(refused.contains("no `[package.metadata.hql]`"), "{refused}");
    }

    #[test]
    fn a_module_is_a_file_or_a_folder_under_the_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("ml")).unwrap();
        std::fs::write(dir.path().join("ml/mod.hql"), "module ml\n").unwrap();
        std::fs::write(dir.path().join("ml/metrics.hql"), "module ml.metrics\n").unwrap();
        assert_eq!(
            module_file(dir.path(), "ml"),
            Some(dir.path().join("ml/mod.hql"))
        );
        assert_eq!(
            module_file(dir.path(), "ml.metrics"),
            Some(dir.path().join("ml/metrics.hql"))
        );
        assert_eq!(module_file(dir.path(), "ml.absent"), None);
    }
}
