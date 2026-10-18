//! Running HQL written inside a document, and building the document it yields.
//!
//! A card can carry a query in a fenced block, a cell. Rendering the card runs
//! each cell against the vault and writes its output as Markdown, so a to-do
//! list or an index is a query rather than a list somebody maintains.
//!
//! A cell is fenced `hql`, `hql#eval` or `hql#query`; the words after that are
//! its attributes: `id=<name>`, `replace`, `after` and `hidden`. A cell with an
//! `id` exports its value under that name to every other cell of the document,
//! and nothing else it binds. The cells form a dependency graph, checked as a
//! whole and evaluated in dependency order, document order breaking ties; a
//! cell that waits on a failed one is skipped. See
//! [HQL-0009](../../doc/proposals/HQL-0009/README.md).
//!
//! Under [`Emit::Replace`] the output takes the cell's place, and the rendered
//! document is a build of the source. Under [`Emit::After`] the cell stays and
//! its output follows it between an opening marker, which records the cell's
//! id, the hash of the code it came from and its status, and [`OUTPUT_CLOSE`].
//! Markdown previews do not display the markers; the next render replaces what
//! is between them, so rendering twice leaves what rendering once did.

use crate::reporting::{Mode, Report, Reports, Severity};
use crate::vault::Vault;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ops::Range;

/// How a line that opens a cell's output under [`Emit::After`] begins.
pub const OUTPUT_OPEN: &str = "<!-- hql:output";
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

/// What became of one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// It produced a value.
    Ok,
    /// It failed: its output is its reports.
    Error,
    /// It waited on a cell that failed, and did not run.
    Skipped,
    /// It was not asked to run: another cell was, or the document did not
    /// check. Its output is left as written.
    NotRun,
}

impl Status {
    /// The word an output marker and the JSON record write.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Skipped => "skipped",
            Self::NotRun => "not-run",
        }
    }
}

/// One cell, as rendering found and left it.
#[derive(Debug, Clone)]
pub struct Cell {
    /// The name its value is read by, when it has one.
    pub id: Option<String>,
    /// Its position among the document's cells, from `0`.
    pub index: usize,
    /// The byte range of its fences in the input, closing fence included.
    pub span: Range<usize>,
    /// The hash of its code and of the cells it reads.
    pub hash: String,
    /// The type of its value, once checked.
    pub ty: Option<String>,
    /// What became of it.
    pub status: Status,
    /// Its output.
    pub markdown: String,
    /// What it had to say, with spans into the document.
    pub reports: Reports,
}

/// What rendering a document produced.
#[derive(Debug, Clone)]
pub struct Rendered {
    /// The document, with every cell's output written.
    pub document: String,
    /// How many cells were run.
    pub blocks: usize,
    /// Whether every cell under [`Emit::Replace`] produced its output. A cell
    /// that failed or did not run is left as written, so the document is not
    /// a complete build.
    pub complete: bool,
    /// Everything the document and every cell had to say, in document order.
    pub reports: Reports,
    /// Every cell, in document order.
    pub cells: Vec<Cell>,
}

/// A cell as written.
struct Source {
    id: Option<String>,
    placement: Emit,
    hidden: bool,
    /// Its fence lines, as written.
    written: Vec<String>,
    body: String,
    /// The byte offset of its body in the document.
    body_at: usize,
    span: Range<usize>,
    /// The output a previous render left after it, marker lines included.
    output: Option<Vec<String>>,
    /// The ids of the cells it reads.
    reads: Vec<usize>,
}

enum Segment {
    Line(String),
    Cell(usize),
}

/// Run every cell in a document and write each one's output.
#[must_use]
pub fn render(source: &str, vault: &Vault, mode: Mode, emit: Emit) -> Rendered {
    match render_cell(source, vault, mode, emit, None) {
        Ok(rendered) => rendered,
        Err(message) => Rendered {
            document: source.to_owned(),
            blocks: 0,
            complete: false,
            reports: std::iter::once(report(Severity::Error, 0..0, message)).collect(),
            cells: Vec::new(),
        },
    }
}

/// Run one cell and the cells it reads, and rewrite only that cell's output;
/// with `None`, run every cell.
///
/// # Errors
///
/// Names a cell id the document does not have.
pub fn render_cell(
    source: &str,
    vault: &Vault,
    mode: Mode,
    emit: Emit,
    only: Option<&str>,
) -> Result<Rendered, String> {
    let mut reports = Reports::new();
    let (segments, mut cells) = scan(source, emit, &mut reports);

    // Ids name values, so two cells may not share one.
    let mut ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut broken = false;
    for (index, cell) in cells.iter().enumerate() {
        let Some(id) = &cell.id else { continue };
        if let Some(first) = ids.get(id) {
            broken = true;
            reports.push(report(
                Severity::Error,
                cell.span.clone(),
                format!("cells {first} and {index} both have the id `{id}`"),
            ));
        } else {
            ids.insert(id.clone(), index);
        }
    }
    let found: Vec<Vec<usize>> = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| reads(cell, &ids, index))
        .collect();
    for (cell, read) in cells.iter_mut().zip(found) {
        cell.reads = read;
    }

    let targets: Vec<usize> = match only {
        None => (0..cells.len()).collect(),
        Some(id) => {
            let &target = ids
                .get(id)
                .ok_or_else(|| format!("the document has no cell with the id `{id}`"))?;
            closure(&cells, target)
        }
    };
    let order = match ordered(&cells, &targets) {
        Ok(order) => order,
        Err(cycle) => {
            broken = true;
            let names: Vec<String> = cycle
                .iter()
                .map(|&index| cells[index].id.clone().unwrap_or_else(|| index.to_string()))
                .map(|name| format!("`{name}`"))
                .collect();
            reports.push(report(
                Severity::Error,
                cells[cycle[0]].span.clone(),
                format!("the cells {} read each other in a cycle", names.join(", ")),
            ));
            Vec::new()
        }
    };

    let hashes = hashes(&cells);
    let mut results: Vec<Cell> = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| Cell {
            id: cell.id.clone(),
            index,
            span: cell.span.clone(),
            hash: hashes[index].clone(),
            ty: None,
            status: Status::NotRun,
            markdown: String::new(),
            reports: Reports::new(),
        })
        .collect();

    // The document is checked as a whole before any cell runs, so a type
    // error in a late cell stops the build before an early one runs.
    let mut programs: Vec<Option<crate::execution::Program>> =
        (0..cells.len()).map(|_| None).collect();
    let mut types: Vec<Option<crate::types::TypeRef>> = vec![None; cells.len()];
    let mut blocks = 0;
    for &index in &order {
        if let Some(waited) = failed_read(&cells[index], &results) {
            skip(&mut results[index], &cells, waited);
            continue;
        }
        let bound: Vec<_> = cells[index]
            .reads
            .iter()
            .filter_map(|&read| Some((cells[read].id.clone()?, types[read].clone()?)))
            .collect();
        blocks += 1;
        let (program, found) = crate::check_cell(&cells[index].body, vault, mode, &bound);
        results[index].reports = shifted(found, cells[index].body_at);
        match program {
            Some(program) => {
                types[index] = Some(program.result.clone());
                results[index].ty = Some(program.result.to_string());
                programs[index] = Some(program);
            }
            None => {
                broken = true;
                results[index].status = Status::Error;
                results[index].markdown = described(&results[index].reports);
            }
        }
    }

    let mut values: Vec<Option<crate::types::Value>> = vec![None; cells.len()];
    if !broken {
        for &index in &order {
            if let Some(waited) = failed_read(&cells[index], &results) {
                skip(&mut results[index], &cells, waited);
                continue;
            }
            let Some(program) = &programs[index] else {
                continue;
            };
            let bound: Vec<_> = cells[index]
                .reads
                .iter()
                .filter_map(|&read| Some((cells[read].id.clone()?, values[read].clone()?)))
                .collect();
            let (value, log) = crate::evaluate_cell(program, vault, bound);
            for entry in shifted(log, cells[index].body_at).iter() {
                results[index].reports.push(entry.clone());
            }
            let value = value.map_err(|diagnostic| {
                let failure: Reports = std::iter::once(Report::of(&diagnostic)).collect();
                for entry in shifted(failure, cells[index].body_at).iter() {
                    results[index].reports.push(entry.clone());
                }
                diagnostic.message()
            });
            let output = value.and_then(|value| {
                let markdown = crate::extensions::format::render(
                    &value,
                    crate::types::TypeConstructor("Hmd"),
                    0,
                    None,
                    &vault.formats,
                );
                values[index] = Some(value);
                markdown
            });
            match output {
                Ok(markdown) => {
                    results[index].status = Status::Ok;
                    results[index].markdown = markdown;
                }
                Err(message) => {
                    results[index].status = Status::Error;
                    if results[index].reports.is_empty() {
                        results[index].reports.push(report(
                            Severity::Error,
                            cells[index].span.clone(),
                            message,
                        ));
                    }
                    results[index].markdown = described(&results[index].reports);
                }
            }
        }
    }

    let (document, complete) = emitted(source, &segments, &cells, &results, only.is_some());
    for result in &results {
        for entry in result.reports.iter() {
            reports.push(entry.clone());
        }
    }
    Ok(Rendered {
        document,
        blocks,
        complete,
        reports,
        cells: results,
    })
}

/// Split a document into lines and cells, attaching to each cell the output a
/// previous render left after it. An output with no cell above it is dropped
/// with a warning.
fn scan(source: &str, emit: Emit, reports: &mut Reports) -> (Vec<Segment>, Vec<Source>) {
    let lines: Vec<&str> = source.lines().collect();
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut at = 0;
    for line in source.split_inclusive('\n') {
        offsets.push(at);
        at += line.len();
    }
    offsets.push(at);
    let mut segments = Vec::new();
    let mut cells: Vec<Source> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim_start().starts_with(OUTPUT_OPEN) {
            let end = output_end(&lines, index);
            reports.push(report(
                Severity::Warning,
                offsets[index]..offsets[end + 1],
                "an output with no cell directly above it is removed".to_owned(),
            ));
            index = end + 1;
            continue;
        }
        let Some(info) = fence(lines[index]) else {
            segments.push(Segment::Line(lines[index].to_owned()));
            index += 1;
            continue;
        };
        let mut close = index + 1;
        while close < lines.len() && fence(lines[close]).is_none() {
            close += 1;
        }
        let last = close.min(lines.len().saturating_sub(1));
        let Some(attributes) = cell_attributes(info) else {
            segments.extend(
                lines[index..=last]
                    .iter()
                    .map(|line| Segment::Line((*line).to_owned())),
            );
            index = close + 1;
            continue;
        };
        let span = offsets[index]..offsets[(last + 1).min(lines.len())];
        let (id, placement, hidden) = attributes.read(emit, &span, reports);
        let body = lines[index + 1..close.min(lines.len())].join("\n");
        let mut cell = Source {
            id,
            placement,
            hidden,
            written: lines[index..=last]
                .iter()
                .map(|line| (*line).to_owned())
                .collect(),
            body,
            body_at: offsets[(index + 1).min(lines.len())],
            span,
            output: None,
            reads: Vec::new(),
        };
        index = close + 1;
        // An output belongs to the cell above it when only blank lines stand
        // between them.
        let mut next = index;
        while next < lines.len() && lines[next].trim().is_empty() {
            next += 1;
        }
        if next < lines.len() && lines[next].trim_start().starts_with(OUTPUT_OPEN) {
            let end = output_end(&lines, next);
            let output: Vec<String> = lines[next..=end]
                .iter()
                .map(|line| (*line).to_owned())
                .collect();
            if let Some(written) = marker_id(&output[0])
                && cell.id.as_deref() != Some(written.as_str())
            {
                reports.push(report(
                    Severity::Warning,
                    offsets[next]..offsets[end + 1],
                    format!(
                        "the output of `{written}` is not below the cell `{written}`, so it is removed"
                    ),
                ));
            } else {
                cell.output = Some(output);
            }
            index = end + 1;
        }
        segments.push(Segment::Cell(cells.len()));
        cells.push(cell);
    }
    (segments, cells)
}

/// The words after a cell fence's kind.
struct Attributes<'a>(Vec<&'a str>);

impl Attributes<'_> {
    fn read(
        &self,
        default: Emit,
        span: &Range<usize>,
        reports: &mut Reports,
    ) -> (Option<String>, Emit, bool) {
        let (mut id, mut placement, mut hidden) = (None, default, false);
        for word in &self.0 {
            match *word {
                "replace" => placement = Emit::Replace,
                "after" => placement = Emit::After,
                "hidden" => hidden = true,
                _ => match word.strip_prefix("id=") {
                    Some(name) if is_identifier(name) => id = Some(name.to_owned()),
                    Some(name) => reports.push(report(
                        Severity::Error,
                        span.clone(),
                        format!("a cell id is a snake_case name, and `{name}` is not one"),
                    )),
                    None => reports.push(report(
                        Severity::Warning,
                        span.clone(),
                        format!("`{word}` is not a cell attribute, so it is ignored"),
                    )),
                },
            }
        }
        (id, placement, hidden)
    }
}

/// The attributes of a fence's info string, or `None` when the fence is not a
/// cell.
fn cell_attributes(info: &str) -> Option<Attributes<'_>> {
    let mut words = info.split_whitespace();
    if !matches!(words.next(), Some("hql" | "hql#eval" | "hql#query")) {
        return None;
    }
    Some(Attributes(words.collect()))
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The ids of other cells a cell reads: every name it uses that is another
/// cell's id, and that it does not bind itself.
fn reads(cell: &Source, ids: &BTreeMap<String, usize>, own: usize) -> Vec<usize> {
    let Ok(tokens) = crate::lexer::scan(&cell.body) else {
        return Vec::new();
    };
    use crate::lexer::Token;
    let mut bound = std::collections::BTreeSet::new();
    let mut used = std::collections::BTreeSet::new();
    for (at, spanned) in tokens.iter().enumerate() {
        let Token::Ident(name) = &spanned.token else {
            continue;
        };
        let before = at.checked_sub(1).map(|before| &tokens[before].token);
        let after = tokens.get(at + 1).map(|after| &after.token);
        // A field, `x.todo`, is not a name.
        if matches!(before, Some(Token::Dot)) {
            continue;
        }
        if matches!(after, Some(Token::Equals)) {
            // `todo = …` at the start of a statement binds; inside a call it
            // names an argument.
            if matches!(before, None | Some(Token::Newline)) {
                bound.insert(name.clone());
            }
            continue;
        }
        used.insert(name.clone());
    }
    used.difference(&bound)
        .filter_map(|name| ids.get(name).copied())
        .filter(|&index| index != own)
        .collect()
}

/// A cell and every cell it reads, directly or through others.
fn closure(cells: &[Source], target: usize) -> Vec<usize> {
    let mut seen = vec![false; cells.len()];
    let mut stack = vec![target];
    while let Some(index) = stack.pop() {
        if std::mem::replace(&mut seen[index], true) {
            continue;
        }
        stack.extend(cells[index].reads.iter().copied());
    }
    (0..cells.len()).filter(|&index| seen[index]).collect()
}

/// The targets in an order where each comes after the cells it reads,
/// document order breaking ties; or the cells on a cycle.
fn ordered(cells: &[Source], targets: &[usize]) -> Result<Vec<usize>, Vec<usize>> {
    let mut placed = vec![false; cells.len()];
    let mut order = Vec::new();
    while order.len() < targets.len() {
        let next = targets
            .iter()
            .copied()
            .find(|&index| !placed[index] && cells[index].reads.iter().all(|&read| placed[read]));
        let Some(next) = next else {
            let waiting: Vec<usize> = targets
                .iter()
                .copied()
                .filter(|&index| !placed[index])
                .collect();
            let on_cycle = waiting
                .iter()
                .copied()
                .filter(|&index| reaches(cells, index, index))
                .collect();
            return Err(on_cycle);
        };
        placed[next] = true;
        order.push(next);
    }
    Ok(order)
}

/// Whether reading from `from` leads back to `to`.
fn reaches(cells: &[Source], from: usize, to: usize) -> bool {
    let mut seen = vec![false; cells.len()];
    let mut stack: Vec<usize> = cells[from].reads.clone();
    while let Some(index) = stack.pop() {
        if index == to {
            return true;
        }
        if !std::mem::replace(&mut seen[index], true) {
            stack.extend(cells[index].reads.iter().copied());
        }
    }
    false
}

/// Each cell's hash: its code and the hashes of the cells it reads, so that
/// editing a cell makes every output downstream of it stale.
fn hashes(cells: &[Source]) -> Vec<String> {
    let mut known: Vec<Option<String>> = vec![None; cells.len()];
    fn hash(cells: &[Source], index: usize, known: &mut [Option<String>], depth: usize) -> String {
        if let Some(hash) = &known[index] {
            return hash.clone();
        }
        // A cycle is reported elsewhere; its hash stops at the source.
        let mut reads: Vec<String> = if depth > cells.len() {
            Vec::new()
        } else {
            cells[index]
                .reads
                .iter()
                .filter_map(|&read| {
                    let id = cells[read].id.clone()?;
                    Some(format!("{id}={}", hash(cells, read, known, depth + 1)))
                })
                .collect()
        };
        reads.sort();
        let mut digest = Sha256::new();
        digest.update(b"hql-cell\0");
        digest.update(normalized(&cells[index].body).as_bytes());
        digest.update(b"\0");
        digest.update(reads.join("\0").as_bytes());
        let full = digest.finalize();
        let hex: String = full.iter().map(|byte| format!("{byte:02x}")).collect();
        let short = hex[..12].to_owned();
        known[index] = Some(short.clone());
        short
    }
    (0..cells.len())
        .map(|index| hash(cells, index, &mut known, 0))
        .collect()
}

/// A cell's source with trailing whitespace stripped from each line and
/// trailing blank lines removed.
fn normalized(source: &str) -> String {
    let lines: Vec<&str> = source.lines().map(str::trim_end).collect();
    let end = lines
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(0, |last| last + 1);
    lines[..end].join("\n")
}

/// The first cell a cell reads that failed or was skipped.
fn failed_read(cell: &Source, results: &[Cell]) -> Option<usize> {
    cell.reads
        .iter()
        .copied()
        .find(|&read| matches!(results[read].status, Status::Error | Status::Skipped))
}

fn skip(result: &mut Cell, cells: &[Source], waited: usize) {
    let name = cells[waited]
        .id
        .clone()
        .unwrap_or_else(|| waited.to_string());
    result.status = Status::Skipped;
    result.markdown = format!("skipped: {name} failed");
}

/// The document with every cell's output written, and whether it is a
/// complete build.
fn emitted(
    source: &str,
    segments: &[Segment],
    cells: &[Source],
    results: &[Cell],
    one_cell: bool,
) -> (String, bool) {
    let mut out: Vec<String> = Vec::new();
    let mut complete = true;
    let mut skip_blank = false;
    for segment in segments {
        let index = match segment {
            Segment::Line(line) => {
                if skip_blank && line.trim().is_empty() {
                    skip_blank = false;
                    continue;
                }
                skip_blank = false;
                out.push(line.clone());
                continue;
            }
            Segment::Cell(index) => *index,
        };
        let (cell, result) = (&cells[index], &results[index]);
        if result.status == Status::NotRun {
            // Left as written: another cell was asked for, or the document
            // did not check.
            out.extend(cell.written.iter().cloned());
            if let Some(output) = &cell.output {
                out.extend(output.iter().cloned());
            }
            // A cell left in a build under `replace` makes it incomplete.
            if !one_cell && cell.placement == Emit::Replace {
                complete = false;
            }
            continue;
        }
        match (cell.placement, result.status) {
            (Emit::Replace, Status::Ok) => {
                if result.markdown.is_empty() || cell.hidden {
                    // Nothing takes the cell's place, so neither does the
                    // blank line that separated it from what follows.
                    skip_blank = true;
                } else {
                    out.extend(result.markdown.lines().map(str::to_owned));
                }
            }
            (Emit::Replace, _) => {
                complete = false;
                out.extend(cell.written.iter().cloned());
            }
            (Emit::After, status) => {
                out.extend(cell.written.iter().cloned());
                if cell.hidden && status == Status::Ok {
                    continue;
                }
                let id = cell
                    .id
                    .as_ref()
                    .map(|id| format!(" id={id}"))
                    .unwrap_or_default();
                out.push(format!(
                    "{OUTPUT_OPEN}{id} hash={} status={} -->",
                    result.hash,
                    status.label()
                ));
                out.extend(result.markdown.lines().map(str::to_owned));
                out.push(OUTPUT_CLOSE.to_owned());
            }
        }
    }
    let mut document = out.join("\n");
    if source.ends_with('\n') && !document.ends_with('\n') {
        document.push('\n');
    }
    (document, complete)
}

/// The line that closes the output opened at `open`, or the last line.
fn output_end(lines: &[&str], open: usize) -> usize {
    lines[open..]
        .iter()
        .position(|line| line.trim() == OUTPUT_CLOSE)
        .map_or(lines.len() - 1, |end| open + end)
}

/// The id an output's opening marker records.
fn marker_id(marker: &str) -> Option<String> {
    marker
        .split_whitespace()
        .find_map(|word| word.strip_prefix("id="))
        .map(str::to_owned)
}

/// A failed cell's output: its reports, one per line.
fn described(reports: &Reports) -> String {
    reports
        .iter()
        .filter(|report| report.severity >= Severity::Error)
        .map(|report: &Report| format!("{}: {}", report.severity.label(), report.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A cell's reports with their spans moved from its body to the document.
fn shifted(reports: Reports, at: usize) -> Reports {
    reports
        .iter()
        .map(|entry| {
            let mut entry = entry.clone();
            entry.span = entry.span.start + at..entry.span.end + at;
            entry
        })
        .collect()
}

fn report(severity: Severity, span: Range<usize>, message: String) -> Report {
    Report {
        severity,
        span,
        stage: "render".to_owned(),
        message,
        code: None,
        label: None,
        ty: None,
        found: None,
        shown: None,
        trace: Vec::new(),
    }
}

/// The info string of a fence line, or `None` when the line is not one.
fn fence(line: &str) -> Option<&str> {
    line.trim_end().strip_prefix("```")
}
