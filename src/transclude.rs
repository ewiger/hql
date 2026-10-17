//! Running HQL written inside a document, and building the document it yields.
//!
//! A card can carry a query in a fenced block, a cell. Rendering the card runs
//! each cell against the vault and writes its output as Markdown, so a to-do
//! list or an index is a query rather than a list somebody maintains.
//!
//! A cell is fenced `hql`, `hql#eval` or `hql#query`. Under [`Emit::Replace`]
//! the output takes the cell's place, and the rendered document is a build of
//! the source. Under [`Emit::After`] the cell stays and its output follows it
//! between [`OUTPUT_OPEN`] and [`OUTPUT_CLOSE`], which Markdown previews do not
//! display; the next render replaces what is between them, so rendering twice
//! leaves what rendering once did. A cell's info string may name the mode for
//! that cell alone: `hql#eval after`.

use crate::reporting::{Mode, Report, Reports};
use crate::vault::Vault;

/// The line that opens a cell's output under [`Emit::After`].
pub const OUTPUT_OPEN: &str = "<!-- hql:output -->";
/// The line that closes it.
pub const OUTPUT_CLOSE: &str = "<!-- /hql:output -->";

/// Where a cell's output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emit {
    /// The output replaces the cell.
    Replace,
    /// The cell stays, and the output follows it between the markers.
    After,
}

impl Emit {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "replace" => Some(Self::Replace),
            "after" => Some(Self::After),
            _ => None,
        }
    }
}

/// What rendering a document produced.
#[derive(Debug, Clone)]
pub struct Rendered {
    /// The document, with every cell's output written.
    pub document: String,
    /// How many cells were run.
    pub blocks: usize,
    /// Whether every cell under [`Emit::Replace`] succeeded. A failed one is
    /// left as written, so the document is not a complete build.
    pub complete: bool,
    /// Everything every cell had to say, in document order.
    pub reports: Reports,
}

/// The mode a fence's info string runs its cell under, or `None` when the
/// fence is not a cell.
fn cell(info: &str, default: Emit) -> Option<Emit> {
    let mut words = info.split_whitespace();
    if !matches!(words.next(), Some("hql" | "hql#eval" | "hql#query")) {
        return None;
    }
    Some(words.find_map(Emit::parse).unwrap_or(default))
}

/// Run every cell in a document and write each one's output.
#[must_use]
pub fn render(source: &str, vault: &Vault, mode: Mode, emit: Emit) -> Rendered {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut reports = Reports::new();
    let mut blocks = 0;
    let mut complete = true;
    let mut index = 0;

    while index < lines.len() {
        let Some(info) = fence(lines[index]) else {
            out.push(lines[index].to_owned());
            index += 1;
            continue;
        };
        let mut close = index + 1;
        while close < lines.len() && fence(lines[close]).is_none() {
            close += 1;
        }
        let last = close.min(lines.len().saturating_sub(1));
        let written = &lines[index..=last];
        let body = lines[index + 1..close.min(lines.len())].join("\n");
        index = close + 1;

        let Some(placement) = cell(info, emit) else {
            out.extend(written.iter().map(|line| (*line).to_owned()));
            continue;
        };

        // Output a previous render left after this cell is stale by
        // construction: the cell is about to produce it again.
        if lines
            .get(index)
            .is_some_and(|line| line.trim() == OUTPUT_OPEN)
            && let Some(end) = lines[index..]
                .iter()
                .position(|line| line.trim() == OUTPUT_CLOSE)
        {
            index += end + 1;
        }

        blocks += 1;
        let outcome = crate::run(&body, vault, mode);
        for report in outcome.reports.iter() {
            reports.push(report.clone());
        }
        let output = match &outcome.value {
            Some(value) => Ok(value.to_markdown()),
            None => Err(outcome
                .reports
                .iter()
                .map(|report: &Report| format!("{}: {}", report.severity.label(), report.message))
                .collect::<Vec<_>>()
                .join("\n")),
        };

        match (placement, output) {
            (Emit::Replace, Ok(markdown)) => {
                if markdown.is_empty() {
                    // Nothing takes the cell's place, so neither does the
                    // blank line that separated it from what follows.
                    if lines.get(index).is_some_and(|line| line.trim().is_empty()) {
                        index += 1;
                    }
                }
                out.extend(markdown.lines().map(str::to_owned));
            }
            (Emit::Replace, Err(_)) => {
                complete = false;
                out.extend(written.iter().map(|line| (*line).to_owned()));
            }
            (Emit::After, Ok(text) | Err(text)) => {
                out.extend(written.iter().map(|line| (*line).to_owned()));
                out.push(OUTPUT_OPEN.to_owned());
                out.extend(text.lines().map(str::to_owned));
                out.push(OUTPUT_CLOSE.to_owned());
            }
        }
    }

    let mut document = out.join("\n");
    if source.ends_with('\n') && !document.ends_with('\n') {
        document.push('\n');
    }
    Rendered {
        document,
        blocks,
        complete,
        reports,
    }
}

/// The info string of a fence line, or `None` when the line is not one.
fn fence(line: &str) -> Option<&str> {
    line.trim_end().strip_prefix("```")
}
