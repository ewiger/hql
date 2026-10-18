//! How much a program is allowed to say before it stops.
//!
//! A run produces a structured log beside its value. The program adds
//! `debug` and `info` entries with the `debug` step; the language adds
//! warnings that change nothing, errors that make a result wrong, and failures
//! that make a result impossible. Which of those stops the run, and which are
//! shown, is a policy rather than a property of the language, so it is
//! configured — by the user's environment, by the vault, and by the individual
//! query, in that increasing order of specificity.

use crate::diagnostics::Diagnostic;
use crate::warnings::Warning;
use std::path::Path;
use std::{env, fs};

/// How serious a report is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A value the program captured to look at.
    Debug,
    /// A value or a fact the program states.
    Info,
    /// Work continued and the result stands.
    Warning,
    /// The result is wrong, but more of the program could still be examined.
    Error,
    /// Nothing further can be examined: the source did not parse, or
    /// evaluation cannot proceed.
    Failure,
}

impl Severity {
    /// The lowercase word used in output and in configuration.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Failure => "failure",
        }
    }

    /// Parse a configured level name. `failure` is not one: a failure is
    /// always shown, so hiding everything below it is not a level.
    #[must_use]
    pub fn parse_level(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// One step of the path evaluation took to an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep {
    /// The step's name.
    pub step: String,
    /// Its one-based position in its pipeline.
    pub position: usize,
    /// The element it was applying a function to, when it was.
    pub element: Option<String>,
}

impl TraceStep {
    fn to_json(&self) -> serde_json::Value {
        let mut entry = serde_json::json!({ "step": self.step, "position": self.position });
        if let Some(element) = &self.element {
            entry["element"] = serde_json::Value::String(element.clone());
        }
        entry
    }
}

/// When a run stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Stop at the first error. The default, because a wrong answer that
    /// keeps going is harder to notice than one that stops.
    #[default]
    Strict,
    /// Report everything findable, and stop only at a failure.
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

/// One thing a run has to say.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// How serious it is.
    pub severity: Severity,
    /// Where in the source it points.
    pub span: std::ops::Range<usize>,
    /// Which stage produced it.
    pub stage: String,
    /// The explanation.
    pub message: String,
    /// The kind of entry, by name, where it has one.
    pub code: Option<String>,
    /// The name a program gave the entry.
    pub label: Option<String>,
    /// The type of the value the entry is about.
    pub ty: Option<String>,
    /// The whole value the entry is about, as data.
    pub found: Option<serde_json::Value>,
    /// The same value as it prints, for a person.
    pub shown: Option<String>,
    /// How evaluation reached the entry, outermost step first.
    pub trace: Vec<TraceStep>,
}

impl Report {
    /// A report for a diagnostic, at the severity its stage implies.
    ///
    /// A syntax error is a failure because nothing after it can be examined.
    /// A type or name error is an error: the rest of the program still can.
    #[must_use]
    pub fn of(diagnostic: &Diagnostic) -> Self {
        let severity = match diagnostic {
            Diagnostic::Syntax { .. } => Severity::Failure,
            Diagnostic::Type { .. } | Diagnostic::Name { .. } => Severity::Error,
            Diagnostic::Runtime { .. } | Diagnostic::Overflow { .. } => Severity::Failure,
        };
        Self {
            severity,
            span: diagnostic.span(),
            stage: diagnostic.stage().to_owned(),
            message: diagnostic.message(),
            ..Self::bare()
        }
    }

    /// An entry with nothing but a severity, for the constructors to fill.
    fn bare() -> Self {
        Self {
            severity: Severity::Warning,
            span: 0..0,
            stage: String::new(),
            message: String::new(),
            code: None,
            label: None,
            ty: None,
            found: None,
            shown: None,
            trace: Vec::new(),
        }
    }

    /// A value the program captured with `debug`.
    #[must_use]
    pub fn captured(capture: Capture) -> Self {
        Self {
            severity: capture.severity,
            span: capture.span,
            stage: "program".to_owned(),
            message: capture.message,
            code: Some("debug".to_owned()),
            label: capture.label,
            ty: Some(capture.ty),
            found: Some(capture.found),
            shown: Some(capture.shown),
            trace: capture.trace,
        }
    }

    /// A warning checking reported, before anything was evaluated.
    #[must_use]
    pub fn of_check_warning(warning: &Warning) -> Self {
        Self {
            stage: "type".to_owned(),
            ..Self::of_warning(warning)
        }
    }

    /// A report for a warning.
    #[must_use]
    pub fn of_warning(warning: &Warning) -> Self {
        Self {
            severity: Severity::Warning,
            span: warning.span.clone(),
            stage: "evaluation".to_owned(),
            message: warning.message.clone(),
            ..Self::bare()
        }
    }

    /// The JSON projection of the report; a field that does not apply is
    /// left out.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let mut entry = serde_json::json!({
            "severity": self.severity.label(),
            "stage": self.stage,
            "message": self.message,
            "span": { "start": self.span.start, "end": self.span.end },
        });
        let text = |value: &Option<String>| value.clone().map(serde_json::Value::String);
        for (key, value) in [
            ("code", text(&self.code)),
            ("label", text(&self.label)),
            ("type", text(&self.ty)),
            ("found", self.found.clone()),
        ] {
            if let Some(value) = value {
                entry[key] = value;
            }
        }
        if !self.trace.is_empty() {
            entry["trace"] = self.trace.iter().map(TraceStep::to_json).collect();
        }
        entry
    }
}

/// Shorten a value's printed form for a line a person reads; the JSON
/// projection of an entry always carries the whole value.
#[must_use]
pub fn shorten(text: &str) -> String {
    const LIMIT: usize = 120;
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let kept: String = text.chars().take(LIMIT).collect();
    format!("{kept}…")
}

/// What a `debug` step captured, before it becomes an entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Capture {
    /// `debug` or `info`.
    pub severity: Severity,
    /// The step's span.
    pub span: std::ops::Range<usize>,
    /// The program's name for the capture.
    pub label: Option<String>,
    /// A short description: the value, or a collection's type and size.
    pub message: String,
    /// The value's type.
    pub ty: String,
    /// The whole value.
    pub found: serde_json::Value,
    /// The value as it prints.
    pub shown: String,
    /// How evaluation reached the step.
    pub trace: Vec<TraceStep>,
}

/// The queue of everything a run had to say, in the order it was said.
///
/// It comes back beside the result rather than instead of it: in collect mode
/// a program can produce reports and still have been worth running, and the
/// caller walks the queue afterwards.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reports {
    items: std::collections::VecDeque<Report>,
}

impl Reports {
    /// An empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a report to the back of the queue.
    pub fn push(&mut self, report: Report) {
        self.items.push_back(report);
    }

    /// Take the next report, oldest first.
    pub fn next_report(&mut self) -> Option<Report> {
        self.items.pop_front()
    }

    /// Walk the queue without consuming it.
    pub fn iter(&self) -> impl Iterator<Item = &Report> {
        self.items.iter()
    }

    /// How many reports are queued.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The most serious severity in the queue.
    #[must_use]
    pub fn worst(&self) -> Option<Severity> {
        self.items.iter().map(|report| report.severity).max()
    }

    /// Whether anything queued stops the result being trustworthy.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.worst().is_some_and(|worst| worst >= Severity::Error)
    }

    /// How many reports of a severity are queued.
    #[must_use]
    pub fn count(&self, severity: Severity) -> usize {
        self.items
            .iter()
            .filter(|report| report.severity == severity)
            .count()
    }

    /// The JSON projection of the queue.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Array(self.items.iter().map(Report::to_json).collect())
    }

    /// The reports at or above a shown level. Hiding one does not remove it:
    /// it was still captured, and the JSON projection has every entry.
    #[must_use]
    pub fn shown(&self, level: Severity) -> Self {
        self.items
            .iter()
            .filter(|report| report.severity >= level)
            .cloned()
            .collect()
    }
}

impl FromIterator<Report> for Reports {
    fn from_iter<I: IntoIterator<Item = Report>>(iterator: I) -> Self {
        Self {
            items: iterator.into_iter().collect(),
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
    fn a_syntax_failure_outranks_a_type_error() {
        let syntax = Diagnostic::syntax(0..1, "no");
        let typing = Diagnostic::typing(0..1, "no");
        assert_eq!(Report::of(&syntax).severity, Severity::Failure);
        assert_eq!(Report::of(&typing).severity, Severity::Error);
        assert!(Severity::Failure > Severity::Error);
        assert!(Severity::Error > Severity::Warning);
    }

    #[test]
    fn modes_parse_by_name_and_default_to_strict() {
        assert_eq!(Mode::parse("Strict"), Some(Mode::Strict));
        assert_eq!(Mode::parse("collect"), Some(Mode::Collect));
        assert_eq!(Mode::parse("loud"), None);
        assert_eq!(Mode::default(), Mode::Strict);
    }

    #[test]
    fn the_queue_keeps_order_and_knows_its_worst() {
        let mut reports = Reports::new();
        reports.push(Report::of_warning(&Warning::new(0..1, "first")));
        reports.push(Report::of(&Diagnostic::typing(2..3, "second")));
        assert_eq!(reports.len(), 2);
        assert_eq!(reports.worst(), Some(Severity::Error));
        assert!(reports.has_errors());
        assert_eq!(reports.next_report().unwrap().message, "first");
        assert_eq!(reports.next_report().unwrap().message, "second");
        assert!(reports.next_report().is_none());
    }

    #[test]
    fn the_query_outranks_every_other_layer() {
        let settings = Settings::resolve(Some(Mode::Collect), None);
        assert_eq!(settings.mode, Mode::Collect);
        assert_eq!(settings.origin, "this query");
    }
}
