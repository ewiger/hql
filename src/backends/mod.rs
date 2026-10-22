//! Execution backends: what a checked program is run by, and which one runs
//! it.
//!
//! A backend receives the checked execution tree and nothing earlier, and is
//! held to the value, the type, the sequence positions and the diagnostic
//! `localhost` gives. Backends are compiled in and registered in one table,
//! as extensions are. Which backend runs a program is host configuration —
//! the query's `--backend`, the vault's `[execution] backend`, the user's
//! `HQL_BACKEND`, then the default — and never syntax, because the answer
//! does not depend on it. See `doc/proposals/HQL-0004/README.md`.

mod localhost;

use crate::diagnostics::Diagnostic;
use crate::execution::Program;
use crate::reporting::{CONFIG_FILE, Report};
use crate::types::Value;
use crate::vault::Vault;
use std::path::Path;
use std::{env, fs};

/// What executes a checked program.
pub(crate) trait Backend: Sync {
    /// Evaluate a checked program against a vault, with further names bound:
    /// the value, or the diagnostic `localhost` would report, and the log the
    /// run wrote.
    fn run(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
    ) -> (Result<Value, Diagnostic>, Vec<Report>);
}

/// One registered backend: its name, what it is, and what runs a program on
/// it.
pub struct Registered {
    /// The name `--backend`, `hql.toml` and `HQL_BACKEND` write.
    pub name: &'static str,
    /// One sentence about how it executes a program.
    pub summary: &'static str,
    backend: &'static dyn Backend,
}

impl std::fmt::Debug for Registered {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Registered")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Registered {
    /// Run a checked program on this backend.
    pub(crate) fn run(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
    ) -> (Result<Value, Diagnostic>, Vec<Report>) {
        self.backend.run(program, vault, bound)
    }
}

/// The standard backend: the in-memory evaluator, always registered, the
/// default and the oracle every other backend is measured against.
pub static LOCALHOST: Registered = Registered {
    name: "localhost",
    summary: "The in-memory evaluator: each step run to completion before the next, \
              one traversal per step, with no fusion, partitioning or parallelism.",
    backend: &localhost::Localhost,
};

/// Every backend compiled into this binary, `localhost` first.
pub static REGISTERED: &[&Registered] = &[&LOCALHOST];

/// The registered backend of a name.
#[must_use]
pub fn named(name: &str) -> Option<&'static Registered> {
    REGISTERED
        .iter()
        .copied()
        .find(|backend| backend.name == name)
}

/// The environment variable a user sets for their own default backend.
pub const ENVIRONMENT: &str = "HQL_BACKEND";

/// The backend in force, and which layer supplied it.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    /// The backend that runs programs.
    pub backend: &'static Registered,
    /// Which layer named it, for `hql config`.
    pub origin: Origin,
}

/// The layer a backend selection came from, most specific first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// `--backend` on this query.
    Query,
    /// The vault's `hql.toml`, `[execution] backend`.
    Vault,
    /// `HQL_BACKEND` in the environment.
    Environment,
    /// Nothing named one.
    Default,
}

impl Origin {
    /// How `hql config` and a refusal name the layer.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Query => "--backend".to_owned(),
            Self::Vault => format!("the vault's {CONFIG_FILE}"),
            Self::Environment => format!("${ENVIRONMENT}"),
            Self::Default => "the default".to_owned(),
        }
    }
}

impl Selection {
    /// `localhost`, selected by default.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            backend: &LOCALHOST,
            origin: Origin::Default,
        }
    }

    /// Resolve the backend from the query, the vault and the environment,
    /// with the precedence the reporting settings use: the most specific
    /// layer that names one wins.
    ///
    /// # Errors
    ///
    /// A name that is not registered fails, naming the layer that supplied
    /// it, so that every program run under it fails rather than running
    /// somewhere else.
    pub fn resolve(query: Option<&str>, vault_root: Option<&Path>) -> Result<Self, String> {
        let from_vault = || {
            let text = fs::read_to_string(vault_root?.join(CONFIG_FILE)).ok()?;
            let parsed: toml::Value = toml::from_str(&text).ok()?;
            Some(
                parsed
                    .get("execution")?
                    .get("backend")?
                    .as_str()?
                    .to_owned(),
            )
        };
        let (name, origin) = if let Some(name) = query {
            (name.to_owned(), Origin::Query)
        } else if let Some(name) = from_vault() {
            (name, Origin::Vault)
        } else if let Ok(name) = env::var(ENVIRONMENT) {
            (name, Origin::Environment)
        } else {
            return Ok(Self::standard());
        };
        Self::of(&name, origin)
    }

    /// The registered backend of a name, supplied by a layer.
    ///
    /// # Errors
    ///
    /// The name is not registered; the message names the layer.
    pub fn of(name: &str, origin: Origin) -> Result<Self, String> {
        let backend = named(name).ok_or_else(|| {
            let known: Vec<String> = REGISTERED
                .iter()
                .map(|backend| format!("`{}`", backend.name))
                .collect();
            format!(
                "no backend is named `{name}`, which {} names; the registered backends are {}",
                origin.label(),
                known.join(", ")
            )
        })?;
        Ok(Self { backend, origin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_is_registered_first_and_is_the_default() {
        assert_eq!(REGISTERED[0].name, "localhost");
        assert!(named("localhost").is_some());
        assert!(named("threads").is_none());
        let selection = Selection::standard();
        assert_eq!(selection.backend.name, "localhost");
        assert_eq!(selection.origin, Origin::Default);
    }

    #[test]
    fn an_unregistered_name_fails_naming_the_layer_that_supplied_it() {
        for (origin, layer) in [
            (Origin::Query, "--backend"),
            (Origin::Vault, "hql.toml"),
            (Origin::Environment, "$HQL_BACKEND"),
        ] {
            let refused = Selection::of("postgres", origin).unwrap_err();
            assert!(refused.contains("`postgres`"), "{refused}");
            assert!(refused.contains(layer), "{refused}");
            assert!(refused.contains("`localhost`"), "{refused}");
        }
    }

    #[test]
    fn the_query_outranks_the_vault() {
        let vault = tempfile::tempdir().unwrap();
        fs::write(
            vault.path().join(CONFIG_FILE),
            "[execution]\nbackend = \"nonesuch\"\n",
        )
        .unwrap();
        let refused = Selection::resolve(None, Some(vault.path())).unwrap_err();
        assert!(refused.contains("hql.toml"), "{refused}");
        let chosen = Selection::resolve(Some("localhost"), Some(vault.path())).unwrap();
        assert_eq!(chosen.origin, Origin::Query);
        fs::write(
            vault.path().join(CONFIG_FILE),
            "[execution]\nbackend = \"localhost\"\n",
        )
        .unwrap();
        let chosen = Selection::resolve(None, Some(vault.path())).unwrap();
        assert_eq!(chosen.origin, Origin::Vault);
        assert_eq!(chosen.backend.name, "localhost");
    }
}
