//! HQL for a page (HQL-0061): `check`, `run` and `version`, called from
//! JavaScript. A program runs in the reading-club vault of the HQL book,
//! held in memory; nothing is read from a file system or a network.

use hql::environment::RunEnvironment;
use hql::playground::{self, MemoryVault};
use include_dir::{Dir, DirEntry, include_dir};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

/// The vault the book's programs read, as its files were when this was built.
static VAULT: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../examples/reading-club");

/// Where the vault is said to be; no directory is behind it.
const ROOT: &str = "/vault";

/// What a run printed.
#[wasm_bindgen(getter_with_clone)]
pub struct Report {
    /// Whether no diagnostic reached `error`.
    pub ok: bool,
    /// Standard output: the value, or for a check the type.
    pub output: String,
    /// Standard error: each diagnostic with the lines it points at.
    pub diagnostics: String,
}

impl From<playground::Report> for Report {
    fn from(report: playground::Report) -> Self {
        Self {
            ok: report.ok,
            output: report.output,
            diagnostics: report.diagnostics,
        }
    }
}

/// Every file of the vault outside a hidden directory, as a walk of the
/// vault on disk lists them.
fn documents(dir: &'static Dir<'static>, found: &mut Vec<(String, String)>) {
    for entry in dir.entries() {
        let hidden = entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'));
        if hidden {
            continue;
        }
        match entry {
            DirEntry::Dir(inner) => documents(inner, found),
            DirEntry::File(file) => {
                if let (Some(path), Some(text)) = (file.path().to_str(), file.contents_utf8()) {
                    found.push((path.to_owned(), text.to_owned()));
                }
            }
        }
    }
}

/// The run environment, assembled on first use and kept for the page's life.
fn environment() -> &'static RunEnvironment {
    static ENVIRONMENT: OnceLock<RunEnvironment> = OnceLock::new();
    ENVIRONMENT.get_or_init(|| {
        let mut found = Vec::new();
        documents(&VAULT, &mut found);
        playground::environment(&MemoryVault::new(ROOT, found))
            .unwrap_or_else(|_| RunEnvironment::bare())
    })
}

/// Parse and type-check a program, as `hql check` does.
#[wasm_bindgen]
#[must_use]
pub fn check(source: &str) -> Report {
    playground::check(source, environment()).into()
}

/// Type-check and evaluate a program, as `hql eval` does.
///
/// JavaScript sees it as `run`: a module cannot export the name `eval`.
#[wasm_bindgen(js_name = run)]
#[must_use]
pub fn eval(source: &str) -> Report {
    playground::eval(source, environment()).into()
}

/// The version of `hql` this was built from.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
