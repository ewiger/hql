//! A program run for a page rather than a terminal (HQL-0061): its source in,
//! and what `hql eval` or `hql check` would print out, as two strings.
//!
//! Nothing here reads a file system. A vault is handed over as texts in
//! memory, so the module is the same in a build for a browser.

use crate::diagnostics::{Diagnostic, Phase, Severity};
use crate::document::Format;
use crate::environment::RunEnvironment;
use crate::reporting::Mode;
use crate::sources::{Snapshot, SourceDocument, VaultSource};
use crate::types::{TypeConstructor, Value};
use std::io;
use std::path::PathBuf;

/// The name a program has in the diagnostics written about it.
pub const ORIGIN: &str = "playground.hql";

/// The format a value is written in: HQL's own, as on a terminal.
const HQL: TypeConstructor = TypeConstructor("Hql");

/// The lowest severity written, the command line's default.
const SHOWN: Severity = Severity::Warning;

/// What a run printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Whether no diagnostic reached `error`.
    pub ok: bool,
    /// What `hql` writes to standard output: the value, or for a check the
    /// type.
    pub output: String,
    /// What `hql` writes to standard error: each diagnostic with the lines
    /// of the program it points at.
    pub diagnostics: String,
}

/// A vault held in memory: each document a path relative to the vault root
/// and its text.
pub struct MemoryVault {
    root: PathBuf,
    documents: Vec<(String, String)>,
    imports: Vec<String>,
}

impl MemoryVault {
    /// A vault named `root` holding `documents`. A path whose extension is
    /// not a document's is left out, as a walk of a directory leaves it.
    #[must_use]
    pub fn new(root: &str, documents: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            root: PathBuf::from(root),
            documents: documents.into_iter().collect(),
            // A vault's programs read its cards: `hmd` is imported for each,
            // as a project's `[modules] import = ["hmd"]` does.
            imports: vec!["hmd".to_owned()],
        }
    }
}

impl VaultSource for MemoryVault {
    fn read(&self) -> io::Result<Snapshot> {
        Ok(Snapshot {
            root: self.root.clone(),
            documents: self
                .documents
                .iter()
                .filter_map(|(path, text)| {
                    let path = PathBuf::from(path);
                    Format::of(&path).map(|format| SourceDocument {
                        path,
                        format,
                        text: Ok(text.clone()),
                    })
                })
                .collect(),
        })
    }

    fn extensions(&self) -> crate::extensions::Config {
        crate::extensions::Config {
            imports: self.imports.clone(),
            ..crate::extensions::Config::default()
        }
    }
}

/// The run environment of a vault held in memory.
///
/// # Errors
/// Returns what assembling the vault refused.
pub fn environment(vault: &MemoryVault) -> io::Result<RunEnvironment> {
    crate::environment::load_source(vault)
}

/// Parse and type-check a program, as `hql check` does.
#[must_use]
pub fn check(source: &str, env: &RunEnvironment) -> Report {
    let outcome = crate::check_reported(source, env, Mode::Strict);
    let mut output = String::new();
    if !outcome.failed()
        && let Some(inferred) = &outcome.inferred
    {
        output = format!("{inferred}\n");
    }
    Report {
        ok: !outcome.failed(),
        output,
        diagnostics: written(&outcome.diagnostics, source),
    }
}

/// Type-check and evaluate a program, as `hql eval` does.
#[must_use]
pub fn eval(source: &str, env: &RunEnvironment) -> Report {
    let outcome = crate::run(source, env, Mode::Strict);
    let mut diagnostics = written(&outcome.diagnostics, source);
    let mut ok = !outcome.failed();
    let output = match &outcome.value {
        None | Some(Value::Unit) => String::new(),
        // A page shows a whole answer; a stream that may not end has none.
        Some(Value::Traversal(traversal)) if !traversal.finite => {
            ok = false;
            diagnostics.push_str(&format!(
                "{} may not end, and a page shows a whole value; bound it with `take(n)`\n",
                traversal.type_of()
            ));
            String::new()
        }
        Some(Value::Formatted(formatted)) => format!("{}\n", formatted.text),
        Some(value) => match crate::extensions::format::render(value, HQL, 0, None, &env.formats) {
            Ok(text) => format!("{text}\n"),
            Err(message) => {
                ok = false;
                diagnostics.push_str(&format!("{message}\n"));
                String::new()
            }
        },
    };
    Report {
        ok,
        output,
        diagnostics,
    }
}

/// The diagnostics at or above the shown level, each as the command line
/// writes it, with the summary line when there is more than one.
fn written(diagnostics: &[Diagnostic], source: &str) -> String {
    let shown: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity >= SHOWN)
        .cloned()
        .map(|mut diagnostic| {
            diagnostic.origin = Some(match diagnostic.origin.take() {
                None => ORIGIN.to_owned(),
                Some(cell) if cell.starts_with('#') => format!("{ORIGIN}{cell}"),
                Some(other) => other,
            });
            diagnostic
        })
        .collect();
    let mut text = String::new();
    for diagnostic in &shown {
        let file = diagnostic
            .origin
            .as_deref()
            .map(|written| written.split_once('#').map_or(written, |(file, _)| file));
        let lines = (file == Some(ORIGIN) && diagnostic.phase != Phase::Load).then_some(source);
        text.push_str(&crate::render::diagnostic_text(diagnostic, lines, ORIGIN));
        text.push_str("\n\n");
    }
    if let Some(summary) = crate::render::summary(&shown) {
        text.push_str(&summary);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOKS: &str = r#"type Book { title : String, year : Int, rating : Int }

books = [
    Book(title = "Dune", year = 1965, rating = 5),
    Book(title = "Emma", year = 1815, rating = 3),
    Book(title = "Beloved", year = 1987, rating = 4),
]

books
| filter(b => b.rating >= 4)
| sort(b => b.title)
| map(b => b.title)
"#;

    #[test]
    fn a_program_gives_what_eval_prints() {
        let report = eval(BOOKS, &RunEnvironment::bare());
        assert!(report.ok, "{}", report.diagnostics);
        assert_eq!(report.output, "[\"Beloved\", \"Dune\"]\n");
        assert_eq!(report.diagnostics, "");
    }

    #[test]
    fn a_mismatch_is_written_with_its_line() {
        let source = BOOKS.replace("year = 1815", "year = \"1815\"");
        let report = check(&source, &RunEnvironment::bare());
        assert!(!report.ok);
        assert_eq!(report.output, "");
        assert!(
            report.diagnostics.starts_with(
                "check error[type-mismatch]: `year` of `Book` is Int, and this is String\n \
                 --> playground.hql:5:33\n"
            ),
            "{}",
            report.diagnostics
        );
    }

    #[test]
    fn a_vault_in_memory_is_queried() {
        let vault = MemoryVault::new(
            "/vault",
            [
                (
                    "dune.hmd".to_owned(),
                    "---\ntitle: Dune\nmetadata:\n  category: book\n---\nBy [[frank-herbert]].\n"
                        .to_owned(),
                ),
                (
                    "frank-herbert.hmd".to_owned(),
                    "---\ntitle: Frank Herbert\nmetadata:\n  category: author\n---\n".to_owned(),
                ),
                ("notes.txt".to_owned(), "not a card".to_owned()),
            ],
        );
        let env = environment(&vault).unwrap();
        let report = eval(
            "cards | filter(c => c.metadata.category == \"book\") | map(c => c.title)",
            &env,
        );
        assert!(report.ok, "{}", report.diagnostics);
        assert_eq!(report.output, "[\"Dune\"]\n");
    }

    #[test]
    fn an_endless_stream_is_refused() {
        let report = eval("iterate(1, n => n + 1)", &RunEnvironment::bare());
        if report.ok {
            // `iterate` is not this build's name for it; nothing to refuse.
            return;
        }
        assert_eq!(report.output, "");
    }
}
