//! Running HQL written inside a document, and putting the answer back.
//!
//! A card can carry a query in a fenced block. Rendering the card runs the
//! query against the vault and transcludes the result beneath it, so a
//! to-do list or an index is a query rather than a list somebody maintains.
//!
//! The console is the first host. A block is marked `hql`, `hql#eval` or
//! `hql#query`, and its answer is written back as an `hql#result` block, which
//! is replaced rather than appended on the next render, so rendering twice
//! leaves the document as rendering once did.

use crate::reporting::{Mode, Report, Reports};
use crate::types::Value;
use crate::vault::Vault;

/// The marker a transcluded answer is written under.
pub const RESULT: &str = "hql#result";

/// What rendering a document produced.
#[derive(Debug, Clone)]
pub struct Rendered {
    /// The document, with every answer transcluded.
    pub document: String,
    /// How many query blocks were run.
    pub blocks: usize,
    /// Everything every block had to say, in document order.
    pub reports: Reports,
}

/// Whether a fence's info string marks a block HQL should run.
fn is_query(info: &str) -> bool {
    let info = info.trim();
    info == "hql" || info == "hql#eval" || info == "hql#query"
}

fn is_result(info: &str) -> bool {
    info.trim() == RESULT
}

/// Run every HQL block in a document and transclude each answer under it.
#[must_use]
pub fn render(source: &str, vault: &Vault, mode: Mode) -> Rendered {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut reports = Reports::new();
    let mut blocks = 0;
    let mut index = 0;

    while index < lines.len() {
        let Some(info) = fence(lines[index]) else {
            out.push(lines[index].to_owned());
            index += 1;
            continue;
        };
        let info = info.to_owned();
        let mut close = index + 1;
        while close < lines.len() && fence(lines[close]).is_none() {
            close += 1;
        }
        let body = lines[index + 1..close.min(lines.len())].join("\n");
        let last = close.min(lines.len().saturating_sub(1));

        if is_result(&info) {
            // An answer a previous render wrote. The block it belongs to has
            // just written a fresh one, so this is stale by construction.
            while out.last().is_some_and(|line| line.is_empty()) {
                out.pop();
            }
            index = close + 1;
            continue;
        }

        for line in &lines[index..=last] {
            out.push((*line).to_owned());
        }
        index = close + 1;

        if !is_query(&info) {
            continue;
        }

        blocks += 1;
        let outcome = crate::run(&body, vault, mode);
        for report in outcome.reports.iter() {
            reports.push(report.clone());
        }
        let answer = match outcome.value {
            Some(value) => present(&value),
            None => outcome
                .reports
                .iter()
                .map(|report: &Report| format!("{}: {}", report.severity.label(), report.message))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        out.push(String::new());
        out.push(format!("```{RESULT}"));
        out.extend(answer.lines().map(std::borrow::ToOwned::to_owned));
        out.push("```".to_owned());
    }

    let mut document = out.join("\n");
    if source.ends_with('\n') && !document.ends_with('\n') {
        document.push('\n');
    }
    Rendered {
        document,
        blocks,
        reports,
    }
}

/// The info string of a fence line, or `None` when the line is not one.
fn fence(line: &str) -> Option<&str> {
    line.trim_end().strip_prefix("```")
}

/// How an answer appears in a document.
///
/// A presenter the query chose is used as written. Otherwise a collection
/// becomes a table, because a document wants the rows rather than a value's
/// one-line rendering.
fn present(value: &Value) -> String {
    match value {
        Value::Presentation(presentation) => presentation.text.clone(),
        Value::Set(..)
        | Value::List(..)
        | Value::Map(_)
        | Value::Ranking(_)
        | Value::Graph(_)
        | Value::Data(_) => value.to_table(),
        other => other.to_string(),
    }
}
