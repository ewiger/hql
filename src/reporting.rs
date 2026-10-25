//! How much a run says, and when it stops (HQL-0045).
//!
//! What a diagnostic is lives in [`crate::diagnostics`]. This module holds the
//! policy the host applies to the run's diagnostic channel: the mode that asks
//! producers for continuation, the lowest severity shown, and the layer each
//! setting came from — the query, the vault's `hql.toml`, the environment, the
//! default, in decreasing order of specificity.

pub use crate::diagnostics::Severity;
use std::path::Path;
use std::{env, fs};

/// How much continuation the run asks its producers for. Whether a producer
/// continues after a diagnostic is the producer's, by phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Decline optional continuation after the first `error`. The default,
    /// because a wrong answer that keeps going is harder to notice than one
    /// that stops.
    #[default]
    Strict,
    /// Ask every producer for as much continuation as it can give.
    Collect,
}

impl Mode {
    /// Parse a configured mode name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "strict" => Some(Self::Strict),
            "collect" | "all" => Some(Self::Collect),
            _ => None,
        }
    }

    /// The name this mode is configured by.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Collect => "collect",
        }
    }
}

/// Where a mode came from, so `hql config` can explain itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The mode in force.
    pub mode: Mode,
    /// Which layer supplied it.
    pub origin: String,
}

/// Which entries are shown, and where that came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    /// The lowest severity shown.
    pub severity: Severity,
    /// Which layer supplied it.
    pub origin: String,
}

impl Level {
    /// Resolve the shown level from the query, the vault and the environment,
    /// with the same precedence as the mode.
    #[must_use]
    pub fn resolve(query: Option<Severity>, vault_root: Option<&Path>) -> Self {
        if let Some(severity) = query {
            return Self {
                severity,
                origin: "this query".to_owned(),
            };
        }
        if let Some(root) = vault_root
            && let Some(severity) = setting(&root.join(CONFIG_FILE), "level")
                .and_then(|name| Severity::parse_level(&name))
        {
            return Self {
                severity,
                origin: format!("the vault's {CONFIG_FILE}"),
            };
        }
        if let Some(severity) = env::var(LEVEL_ENVIRONMENT)
            .ok()
            .and_then(|value| Severity::parse_level(&value))
        {
            return Self {
                severity,
                origin: format!("${LEVEL_ENVIRONMENT}"),
            };
        }
        Self {
            severity: Severity::Warning,
            origin: "the default".to_owned(),
        }
    }
}

/// The environment variable a user sets for their own default.
pub const ENVIRONMENT: &str = "HQL_REPORT";
/// The environment variable a user sets for their own shown level.
pub const LEVEL_ENVIRONMENT: &str = "HQL_REPORT_LEVEL";
/// The file a vault puts its own default in.
pub const CONFIG_FILE: &str = "hql.toml";

impl Settings {
    /// Resolve the mode from the query, the vault and the environment.
    ///
    /// The most specific layer that states a mode wins: a flag on this query,
    /// then the vault's file, then the user's environment, then the default.
    #[must_use]
    pub fn resolve(query: Option<Mode>, vault_root: Option<&Path>) -> Self {
        if let Some(mode) = query {
            return Self {
                mode,
                origin: "this query".to_owned(),
            };
        }
        if let Some(root) = vault_root
            && let Some(mode) = from_file(&root.join(CONFIG_FILE))
        {
            return Self {
                mode,
                origin: format!("the vault's {CONFIG_FILE}"),
            };
        }
        if let Some(mode) = env::var(ENVIRONMENT)
            .ok()
            .and_then(|value| Mode::parse(&value))
        {
            return Self {
                mode,
                origin: format!("${ENVIRONMENT}"),
            };
        }
        Self {
            mode: Mode::default(),
            origin: "the default".to_owned(),
        }
    }
}

/// Read `[report] mode = "…"` from a vault's configuration file.
fn from_file(path: &Path) -> Option<Mode> {
    Mode::parse(&setting(path, "mode")?)
}

/// Read one `[report]` setting from a vault's configuration file.
fn setting(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    Some(parsed.get("report")?.get(key)?.as_str()?.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_parse_by_name_and_default_to_strict() {
        assert_eq!(Mode::parse("Strict"), Some(Mode::Strict));
        assert_eq!(Mode::parse("collect"), Some(Mode::Collect));
        assert_eq!(Mode::parse("loud"), None);
        assert_eq!(Mode::default(), Mode::Strict);
    }

    #[test]
    fn the_query_outranks_every_other_layer() {
        let settings = Settings::resolve(Some(Mode::Collect), None);
        assert_eq!(settings.mode, Mode::Collect);
        assert_eq!(settings.origin, "this query");
    }
}
