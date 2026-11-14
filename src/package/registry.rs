//! The package registry: the file that names the packages `hql package
//! install <name>` reaches, and the fetch of an entry's source.
//!
//! The registry is `hql-packages.yaml` of the `hql-extensions` repository.
//! Every entry is pinned: an official one to a tag of that repository, a
//! known one to a crates.io version or a git revision.

use super::manifest::Source;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// Where the registry is fetched from.
pub const URL: &str =
    "https://raw.githubusercontent.com/ewiger/hql-extensions/main/hql-packages.yaml";

/// The repository official packages are checked out from.
pub const REPOSITORY: &str = "https://github.com/ewiger/hql-extensions";

/// The environment variable naming another registry: a URL or a file.
pub const REGISTRY_ENVIRONMENT: &str = "HQL_REGISTRY";

/// The environment variable naming another repository of official
/// packages: a URL or a directory.
pub const REPOSITORY_ENVIRONMENT: &str = "HQL_REGISTRY_REPOSITORY";

/// The directory of the registry's cache under the HQL home.
pub const CACHE_DIRECTORY: &str = "registry";

/// The directory of fetched sources under the HQL home.
pub const SOURCE_DIRECTORY: &str = "src";

/// The registry's file name.
pub const FILE: &str = "hql-packages.yaml";

/// How long a cached registry is used without a fetch.
const FRESH: Duration = Duration::from_secs(24 * 60 * 60);

/// Where an entry's crate is fetched from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A crate under `crates/` of the registry's repository, at the tag
    /// `<crate>-v<version>`.
    Official {
        /// The version the entry pins.
        version: String,
    },
    /// A crate on crates.io, at a version.
    CratesIo {
        /// The version the entry pins.
        version: String,
    },
    /// A crate at the root of a git repository, at a revision.
    Git {
        /// The repository.
        url: String,
        /// The revision the entry pins.
        rev: String,
    },
}

/// One package of the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The package's name.
    pub name: String,
    /// Its crate.
    pub krate: String,
    /// Where its crate is fetched from.
    pub origin: Origin,
    /// The `hql` versions it builds against, a Cargo requirement.
    pub hql: String,
    /// The modules it provides.
    pub provides: Vec<String>,
    /// What it does, in a line.
    pub description: String,
}

impl Entry {
    /// `official` or `known`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self.origin {
            Origin::Official { .. } => "official",
            Origin::CratesIo { .. } | Origin::Git { .. } => "known",
        }
    }

    /// The version or revision the entry pins.
    #[must_use]
    pub fn pinned(&self) -> &str {
        match &self.origin {
            Origin::Official { version } | Origin::CratesIo { version } => version,
            Origin::Git { rev, .. } => rev,
        }
    }

    /// Whether the entry admits a version of `hql`.
    ///
    /// # Errors
    ///
    /// Returns the refusal naming both, or why the requirement does not
    /// parse.
    pub fn admits(&self, hql: &str) -> Result<(), String> {
        let requirement = semver::VersionReq::parse(&self.hql).map_err(|error| {
            format!(
                "package `{}` requires hql `{}`: {error}",
                self.name, self.hql
            )
        })?;
        let running = semver::Version::parse(hql).map_err(|error| format!("hql {hql}: {error}"))?;
        if requirement.matches(&running) {
            Ok(())
        } else {
            Err(format!(
                "package `{}` requires hql {}; this is hql {hql}",
                self.name, self.hql
            ))
        }
    }
}

/// A registry: its entries, where it came from and when it was fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    /// The packages, in the order written.
    pub entries: Vec<Entry>,
    /// The URL or file it was read from.
    pub origin: String,
    /// When the copy in use was fetched.
    pub fetched: SystemTime,
}

/// Read a registry file's text.
///
/// # Errors
///
/// Returns why the text is no registry of format 1: it does not parse, it
/// states another format, an entry lacks a key, or two entries share a name
/// or a module.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    use serde_yaml_ng::Value;
    let document: Value =
        serde_yaml_ng::from_str(text).map_err(|error| format!("the registry: {error}"))?;
    let format = document.get("registry").and_then(Value::as_u64);
    if format != Some(1) {
        return Err(format!(
            "the registry states format {}; this hql reads format 1",
            format.map_or_else(|| "none".to_owned(), |format| format.to_string())
        ));
    }
    let listed = document
        .get("packages")
        .and_then(Value::as_sequence)
        .ok_or("the registry has no `packages` list")?;
    let mut entries: Vec<Entry> = Vec::new();
    for written in listed {
        let text = |key: &str| written.get(key).and_then(Value::as_str).map(str::to_owned);
        let name = text("name").ok_or("a registry entry has no `name`")?;
        let required = |key: &str| {
            text(key).ok_or_else(|| format!("the registry entry `{name}` has no `{key}`"))
        };
        let origin = match required("kind")?.as_str() {
            "official" => Origin::Official {
                version: required("version")?,
            },
            "known" => {
                let source = written
                    .get("source")
                    .ok_or_else(|| format!("the registry entry `{name}` has no `source`"))?;
                let field = |key: &str| source.get(key).and_then(Value::as_str).map(str::to_owned);
                match (field("crates-io"), field("git"), field("rev")) {
                    (Some(version), None, None) => Origin::CratesIo { version },
                    (None, Some(url), Some(rev)) => Origin::Git { url, rev },
                    _ => {
                        return Err(format!(
                            "the registry entry `{name}` is pinned by `crates-io` and a version, \
                             or by `git` and a `rev`"
                        ));
                    }
                }
            }
            other => {
                return Err(format!(
                    "the registry entry `{name}` is of kind `{other}`; a package is `official` \
                     or `known`"
                ));
            }
        };
        let provides: Vec<String> = written
            .get("provides")
            .and_then(Value::as_sequence)
            .and_then(|modules| {
                modules
                    .iter()
                    .map(|module| module.as_str().map(str::to_owned))
                    .collect()
            })
            .ok_or_else(|| format!("the registry entry `{name}` has no `provides` list"))?;
        if entries.iter().any(|held| held.name == name) {
            return Err(format!("the registry lists `{name}` twice"));
        }
        if let Some((module, owner)) = provides.iter().find_map(|module| {
            entries
                .iter()
                .find(|held| held.provides.contains(module))
                .map(|held| (module, &held.name))
        }) {
            return Err(format!(
                "the registry entries `{owner}` and `{name}` both provide `{module}`"
            ));
        }
        entries.push(Entry {
            krate: required("crate")?,
            hql: required("hql")?,
            description: required("description")?,
            origin,
            provides,
            name,
        });
    }
    Ok(entries)
}

/// The registry in use: `$HQL_REGISTRY`, or the default.
fn address() -> String {
    std::env::var(REGISTRY_ENVIRONMENT).unwrap_or_else(|_| URL.to_owned())
}

/// The repository official packages are checked out from.
fn repository() -> String {
    std::env::var(REPOSITORY_ENVIRONMENT).unwrap_or_else(|_| REPOSITORY.to_owned())
}

/// The text at a registry address: a URL fetched, or a file read.
fn fetch(address: &str) -> Result<String, String> {
    if address.starts_with("http://") || address.starts_with("https://") {
        ureq::get(address)
            .call()
            .map_err(|error| format!("{address}: {error}"))?
            .into_string()
            .map_err(|error| format!("{address}: {error}"))
    } else {
        std::fs::read_to_string(address).map_err(|error| format!("{address}: {error}"))
    }
}

/// The registry, from its cache under the HQL home where that is less than
/// a day old or the fetch fails, and fetched otherwise; `refresh` fetches
/// whatever the cache holds.
///
/// # Errors
///
/// Returns why no registry can be read: nothing is cached and the fetch
/// fails, or the text is no registry.
pub fn load(home: &Path, refresh: bool) -> Result<Registry, String> {
    let address = address();
    let cache = home.join(CACHE_DIRECTORY).join(FILE);
    let cached = std::fs::metadata(&cache)
        .and_then(|metadata| metadata.modified())
        .ok();
    let fresh = cached.is_some_and(|fetched| {
        SystemTime::now()
            .duration_since(fetched)
            .is_ok_and(|age| age < FRESH)
    });
    if fresh && !refresh {
        let text = std::fs::read_to_string(&cache)
            .map_err(|error| format!("{}: {error}", cache.display()))?;
        return Ok(Registry {
            entries: parse(&text)?,
            origin: address,
            fetched: cached.unwrap_or_else(SystemTime::now),
        });
    }
    match fetch(&address) {
        Ok(text) => {
            let entries = parse(&text)?;
            if let Some(directory) = cache.parent() {
                std::fs::create_dir_all(directory)
                    .map_err(|error| format!("{}: {error}", directory.display()))?;
            }
            std::fs::write(&cache, &text)
                .map_err(|error| format!("{}: {error}", cache.display()))?;
            Ok(Registry {
                entries,
                origin: address,
                fetched: SystemTime::now(),
            })
        }
        // An old copy is better than none: the fetch failed, not the
        // registry.
        Err(failed) => match (cached, std::fs::read_to_string(&cache)) {
            (Some(fetched), Ok(text)) if !refresh => Ok(Registry {
                entries: parse(&text)?,
                origin: address,
                fetched,
            }),
            _ => Err(format!("the package registry cannot be read: {failed}")),
        },
    }
}

impl Registry {
    /// The entry of a name.
    ///
    /// # Errors
    ///
    /// Returns that the registry lists no such package.
    pub fn entry(&self, name: &str) -> Result<&Entry, String> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| {
                format!(
                    "the registry lists no package `{name}`: `hql package search <text>` finds \
                     what it lists, and `hql package install --path <dir>` installs a crate \
                     on this machine"
                )
            })
    }

    /// The entries whose name, modules or description hold a text.
    #[must_use]
    pub fn search(&self, text: &str) -> Vec<&Entry> {
        let wanted = text.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&wanted)
                    || entry.description.to_lowercase().contains(&wanted)
                    || entry
                        .provides
                        .iter()
                        .any(|module| module.to_lowercase().contains(&wanted))
            })
            .collect()
    }
}

/// Run a command of the fetch, naming it where it fails.
fn run(command: &mut Command, what: &str) -> Result<(), String> {
    let status = command
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("{what}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed"))
    }
}

/// The directory a package's source is kept in under the HQL home.
#[must_use]
pub fn source_directory(home: &Path, package: &str) -> PathBuf {
    home.join(SOURCE_DIRECTORY).join(package)
}

/// The crate of a fetched package: under `crates/` of a checkout of the
/// registry's repository, or the fetched directory itself.
#[must_use]
pub fn crate_directory(home: &Path, package: &str, krate: &str) -> PathBuf {
    let fetched = source_directory(home, package);
    let nested = fetched.join("crates").join(krate);
    if nested.join("Cargo.toml").is_file() {
        nested
    } else {
        fetched
    }
}

/// Fetch an entry's source into the HQL home, replacing what was there, and
/// return the crate's directory and the record of where it came from.
/// `version` picks another version of an official package or of one on
/// crates.io than the one the entry pins.
///
/// # Errors
///
/// Returns why the source cannot be fetched: no `git`, a tag or a version
/// that does not exist, a download that fails.
pub fn fetch_source(
    home: &Path,
    registry: &Registry,
    entry: &Entry,
    version: Option<&str>,
) -> Result<(PathBuf, Source), String> {
    let destination = source_directory(home, &entry.name);
    if destination.exists() {
        std::fs::remove_dir_all(&destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
    }
    if let Some(above) = destination.parent() {
        std::fs::create_dir_all(above).map_err(|error| format!("{}: {error}", above.display()))?;
    }
    let git = |what: &str| {
        format!(
            "fetching package `{}` needs `git` ({what}), and it is not on PATH or the fetch",
            entry.name
        )
    };
    let tag = match &entry.origin {
        Origin::Official { version: pinned } => {
            let tag = format!("{}-v{}", entry.krate, version.unwrap_or(pinned));
            // The tag alone, and of it the crate and the workspace's root
            // files alone.
            run(
                Command::new("git")
                    .args(["clone", "--quiet", "--depth", "1", "--sparse", "--branch"])
                    .arg(&tag)
                    .arg(repository())
                    .arg(&destination),
                &git(&format!("the tag {tag}")),
            )?;
            run(
                Command::new("git")
                    .arg("-C")
                    .arg(&destination)
                    .args(["sparse-checkout", "set"])
                    .arg(format!("crates/{}", entry.krate)),
                &git("a sparse checkout"),
            )?;
            tag
        }
        Origin::Git { url, rev } => {
            if version.is_some_and(|wanted| wanted != rev) {
                return Err(format!(
                    "package `{}` is pinned to the revision {rev}",
                    entry.name
                ));
            }
            run(
                Command::new("git")
                    .args(["clone", "--quiet"])
                    .arg(url)
                    .arg(&destination),
                &git("a clone"),
            )?;
            run(
                Command::new("git")
                    .arg("-C")
                    .arg(&destination)
                    .args(["checkout", "--quiet"])
                    .arg(rev),
                &git(&format!("the revision {rev}")),
            )?;
            rev.clone()
        }
        Origin::CratesIo { version: pinned } => {
            let wanted = version.unwrap_or(pinned);
            let archive = format!(
                "https://static.crates.io/crates/{krate}/{krate}-{wanted}.crate",
                krate = entry.krate
            );
            let mut bytes = Vec::new();
            ureq::get(&archive)
                .call()
                .map_err(|error| format!("{archive}: {error}"))?
                .into_reader()
                .read_to_end(&mut bytes)
                .map_err(|error| format!("{archive}: {error}"))?;
            std::fs::create_dir_all(&destination)
                .map_err(|error| format!("{}: {error}", destination.display()))?;
            let file = destination.join("crate.tar.gz");
            std::fs::write(&file, bytes).map_err(|error| format!("{}: {error}", file.display()))?;
            // A `.crate` is a gzipped tar holding one directory.
            run(
                Command::new("tar")
                    .arg("-xzf")
                    .arg(&file)
                    .arg("-C")
                    .arg(&destination)
                    .args(["--strip-components", "1"]),
                &format!("unpacking {archive} with `tar`"),
            )?;
            let _ = std::fs::remove_file(&file);
            format!("{}-{wanted}", entry.krate)
        }
    };
    Ok((
        crate_directory(home, &entry.name, &entry.krate),
        Source::Registry {
            registry: registry.origin.clone(),
            tag,
        },
    ))
}

use std::io::Read as _;

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = "\
registry: 1
packages:
  - name: ml
    crate: hql-ext-ml
    kind: official
    version: 0.1.0
    hql: \">=0.7.0, <0.8.0\"
    provides: [ml, ml.metrics]
    description: Datasets and fitted models as values
  - name: fancysql
    crate: hql-ext-fancysql
    kind: known
    source: { crates-io: \"1.2.0\" }
    hql: \">=0.7.0, <0.8.0\"
    provides: [fancysql]
    description: A dialect
  - name: geo
    crate: hql-ext-geo
    kind: known
    source: { git: \"https://example.org/geo\", rev: abc123 }
    hql: \"0.7\"
    provides: [geo]
    description: Distances on a sphere
";

    #[test]
    fn a_registry_lists_official_and_known_packages_each_pinned() {
        let entries = parse(REGISTRY).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].kind(), "official");
        assert_eq!(entries[0].pinned(), "0.1.0");
        assert_eq!(entries[0].provides, ["ml", "ml.metrics"]);
        assert_eq!(
            entries[1].origin,
            Origin::CratesIo {
                version: "1.2.0".to_owned()
            }
        );
        assert_eq!(entries[2].kind(), "known");
        assert_eq!(entries[2].pinned(), "abc123");
    }

    #[test]
    fn an_entry_admits_the_hql_versions_its_requirement_names() {
        let entries = parse(REGISTRY).unwrap();
        assert_eq!(entries[0].admits("0.7.3"), Ok(()));
        assert_eq!(
            entries[0].admits("0.8.0"),
            Err("package `ml` requires hql >=0.7.0, <0.8.0; this is hql 0.8.0".to_owned())
        );
    }

    #[test]
    fn a_registry_of_another_format_or_with_a_shared_name_is_refused() {
        let newer = parse("registry: 2\npackages: []\n").unwrap_err();
        assert!(
            newer.contains("format 2; this hql reads format 1"),
            "{newer}"
        );
        let twice = parse(&REGISTRY.replace("name: fancysql", "name: ml")).unwrap_err();
        assert!(twice.contains("lists `ml` twice"), "{twice}");
        let shared =
            parse(&REGISTRY.replace("provides: [fancysql]", "provides: [ml.metrics]")).unwrap_err();
        assert!(
            shared.contains("`ml` and `fancysql` both provide `ml.metrics`"),
            "{shared}"
        );
        let unpinned =
            parse(&REGISTRY.replace("source: { crates-io: \"1.2.0\" }", "source: {}")).unwrap_err();
        assert!(unpinned.contains("is pinned by"), "{unpinned}");
        let keyless = parse(&REGISTRY.replace("    crate: hql-ext-ml\n", "")).unwrap_err();
        assert!(keyless.contains("`ml` has no `crate`"), "{keyless}");
    }

    #[test]
    fn a_search_reads_names_modules_and_descriptions() {
        let registry = Registry {
            entries: parse(REGISTRY).unwrap(),
            origin: "test".to_owned(),
            fetched: SystemTime::now(),
        };
        let names = |text: &str| -> Vec<&str> {
            registry
                .search(text)
                .iter()
                .map(|entry| entry.name.as_str())
                .collect()
        };
        assert_eq!(names("metrics"), ["ml"]);
        assert_eq!(names("SPHERE"), ["geo"]);
        assert_eq!(names("fancy"), ["fancysql"]);
        assert!(names("absent").is_empty());
        assert!(registry.entry("absent").is_err());
    }
}
