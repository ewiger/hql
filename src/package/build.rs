//! Building a package from its crate, and installing the result.
//!
//! One function does both, [`install`], whatever asked: `hql package
//! install`, `hql package rebuild`, or a crate `[packages] paths` lists
//! whose build is missing or older than its source.

use super::manifest::{Crate, Installed, Source, module_file};
use crate::ext;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The dependency versions this build of `hql` was resolved to. A package
/// built without a lock file of its own starts from them, so the crates the
/// two share resolve alike.
const LOCK: &str = include_str!("../../Cargo.lock");

/// The environment variable naming a checkout of `hql` to build packages
/// against, in place of the released crate of this version.
pub const SOURCE_ENVIRONMENT: &str = "HQL_SOURCE";

/// The checkout of `hql` a package is built against, where this build is
/// not a released one: `$HQL_SOURCE`, or the checkout this binary was
/// compiled from while it is still there.
fn checkout() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os(SOURCE_ENVIRONMENT) {
        return Some(PathBuf::from(named));
    }
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry = compiled
        .components()
        .any(|part| part.as_os_str() == "registry");
    (!registry && compiled.join("Cargo.toml").is_file()).then(|| compiled.to_path_buf())
}

/// The number of the compiler this build was compiled with, `1.95.0`.
fn toolchain() -> &'static str {
    ext::RUSTC.split_whitespace().next().unwrap_or(ext::RUSTC)
}

/// A `cargo` invocation with the compiler of this build selected, where
/// `cargo` is `rustup`'s.
fn cargo() -> Command {
    let mut command = Command::new("cargo");
    command.env("RUSTUP_TOOLCHAIN", toolchain());
    // What a `cargo` running this process set for its own build.
    for inherited in [
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTC",
    ] {
        command.env_remove(inherited);
    }
    command
}

/// Whether the crate accepts this version of `hql`.
fn accepts(package: &Crate) -> Result<(), String> {
    let Some(written) = &package.requirement else {
        return Err(format!(
            "package `{}` does not depend on `hql`: its crate implements an extension against \
             `hql::ext`",
            package.package
        ));
    };
    let requirement = semver::VersionReq::parse(written).map_err(|error| {
        format!(
            "package `{}` requires hql `{written}`: {error}",
            package.package
        )
    })?;
    let running =
        semver::Version::parse(ext::HQL).map_err(|error| format!("hql {}: {error}", ext::HQL))?;
    if requirement.matches(&running) {
        Ok(())
    } else {
        Err(format!(
            "package `{}` requires hql {written}; this is hql {}",
            package.package,
            ext::HQL
        ))
    }
}

/// Copy a directory and everything under it.
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let failed = |path: &Path, error: std::io::Error| format!("{}: {error}", path.display());
    std::fs::create_dir_all(to).map_err(|error| failed(to, error))?;
    for entry in std::fs::read_dir(from).map_err(|error| failed(from, error))? {
        let entry = entry.map_err(|error| failed(from, error))?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            std::fs::copy(&source, &target).map_err(|error| failed(&source, error))?;
        }
    }
    Ok(())
}

/// Where `cargo` puts what it builds for a crate, and the version it
/// resolves for the crate.
fn metadata(package: &Crate, patch: &[String]) -> Result<(PathBuf, String), String> {
    let output = cargo()
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(package.dir.join("Cargo.toml"))
        .args(patch)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("running `cargo metadata`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed for {}",
            package.dir.display()
        ));
    }
    let read: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("reading `cargo metadata`: {error}"))?;
    let target = read
        .get("target_directory")
        .and_then(serde_json::Value::as_str)
        .ok_or("`cargo metadata` names no target directory")?;
    let version = read
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|held| held.get("name").and_then(serde_json::Value::as_str) == Some(&package.name))
        .and_then(|held| held.get("version"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&package.version);
    Ok((PathBuf::from(target), version.to_owned()))
}

/// Build the package of a crate with `cargo` against this `hql`, and
/// install it into `destination`: the library, the module root and
/// `package.toml`. What was there before is replaced.
///
/// # Errors
///
/// Returns why the package cannot be built or installed: the directory is
/// no package's crate, the crate excludes this version of `hql`, no `cargo`
/// is on `PATH`, the build fails, or a module the crate provides has no
/// file under its module root.
pub fn install(crate_dir: &Path, destination: &Path, source: Source) -> Result<Installed, String> {
    let package = Crate::read(crate_dir)?;
    accepts(&package)?;
    for module in &package.provides {
        if module_file(&package.modules, module).is_none() {
            return Err(format!(
                "package `{}` provides `{module}`, which has no file under {}",
                package.package,
                package.modules.display()
            ));
        }
    }
    let present = cargo()
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !present.is_ok_and(|status| status.success()) {
        return Err(format!(
            "building package `{}` needs `cargo` with rustc {}, and none is on PATH: install \
             Rust with rustup (https://rustup.rs)",
            package.package,
            toolchain()
        ));
    }
    let manifest = package.dir.join("Cargo.toml");
    // The crates the package shares with this build resolve as this build's
    // do, unless the crate keeps a lock file of its own.
    let lock = package.dir.join("Cargo.lock");
    let workspace_lock = package
        .dir
        .ancestors()
        .skip(1)
        .any(|above| above.join("Cargo.lock").is_file() && above.join("Cargo.toml").is_file());
    if !lock.is_file() && !workspace_lock {
        std::fs::write(&lock, LOCK).map_err(|error| format!("{}: {error}", lock.display()))?;
    }
    // This build of `hql` is the one the package compiles against: the
    // checkout it came from, or the released crate of exactly this version.
    let patch: Vec<String> = match checkout() {
        Some(path) => vec![
            "--config".to_owned(),
            format!("patch.crates-io.hql.path={:?}", path.display().to_string()),
        ],
        None => {
            let pinned = cargo()
                .args(["update", "--package", "hql", "--precise", ext::HQL])
                .arg("--manifest-path")
                .arg(&manifest)
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|error| format!("running `cargo update`: {error}"))?;
            if !pinned.success() {
                return Err(format!(
                    "package `{}` cannot be built against hql {}: `cargo update --package hql \
                     --precise {}` failed",
                    package.package,
                    ext::HQL,
                    ext::HQL
                ));
            }
            Vec::new()
        }
    };
    let built = cargo()
        .args(["build", "--release", "--lib"])
        .arg("--manifest-path")
        .arg(&manifest)
        .args(&patch)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("running `cargo build`: {error}"))?;
    if !built.success() {
        return Err(format!("package `{}` does not build", package.package));
    }
    let (target, version) = metadata(&package, &patch)?;
    let library = target.join("release").join(package.library());
    if !library.is_file() {
        return Err(format!(
            "the build of package `{}` left no {}: its crate needs `crate-type = [\"cdylib\", \
             \"rlib\"]` under `[lib]`",
            package.package,
            library.display()
        ));
    }
    let failed = |path: &Path, error: std::io::Error| format!("{}: {error}", path.display());
    if destination.exists() {
        std::fs::remove_dir_all(destination).map_err(|error| failed(destination, error))?;
    }
    std::fs::create_dir_all(destination).map_err(|error| failed(destination, error))?;
    std::fs::copy(&library, destination.join(package.library()))
        .map_err(|error| failed(&library, error))?;
    copy_tree(&package.modules, &destination.join("modules"))?;
    let installed = Installed {
        name: package.package.clone(),
        krate: package.name.clone(),
        version,
        provides: package.provides.clone(),
        source,
        hql: ext::HQL.to_owned(),
        rustc: ext::RUSTC.to_owned(),
        target: ext::TARGET.to_owned(),
        library: package.library(),
        built: jiff::Timestamp::now().to_string(),
    };
    installed.write(destination)?;
    Ok(installed)
}

/// The newest modification among a crate's manifest, its sources and its
/// module root.
fn newest(package: &Crate) -> Option<std::time::SystemTime> {
    fn walk(path: &Path, newest: &mut Option<std::time::SystemTime>) {
        let Ok(metadata) = std::fs::metadata(path) else {
            return;
        };
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
                walk(&entry.path(), newest);
            }
        } else if let Ok(modified) = metadata.modified() {
            *newest = Some(newest.map_or(modified, |held| held.max(modified)));
        }
    }
    let mut found = None;
    walk(&package.dir.join("Cargo.toml"), &mut found);
    walk(&package.dir.join("src"), &mut found);
    walk(&package.modules, &mut found);
    found
}

/// The installed build of a crate `[packages] paths` lists, built first
/// where there is none, where it is older than the crate's source, or where
/// it was built against another `hql`: it lives under the crate's own
/// `target/`.
///
/// # Errors
///
/// Returns why the crate is no package or does not build.
pub fn local(crate_dir: &Path) -> Result<PathBuf, String> {
    let package = Crate::read(crate_dir)?;
    let destination = package
        .dir
        .join("target")
        .join("hql")
        .join(&package.package);
    let record = destination.join(super::manifest::INSTALLED);
    let fresh = Installed::read(&destination).is_ok_and(|held| held.stale().is_none())
        && std::fs::metadata(&record)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|built| newest(&package).is_none_or(|changed| changed <= built));
    if !fresh {
        eprintln!(
            "hql: building package `{}` from {}",
            package.package,
            package.dir.display()
        );
        install(
            &package.dir,
            &destination,
            Source::Path(package.dir.clone()),
        )?;
    }
    Ok(destination)
}
