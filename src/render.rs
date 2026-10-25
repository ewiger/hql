//! `DiagnosticText`: a diagnostic rendered for a person (HQL-0045).
//!
//! The byte ranges a diagnostic carries are the machine-readable form and
//! stay that way; rendering them as a line, a column and a caret is a
//! presentation, a separate and testable step rather than something only the
//! binary can do. The heading names the phase, the severity and the kind:
//! `check error[type-mismatch]: …`.

use crate::diagnostics::Diagnostic;

/// A one-based line and column pair, counted in characters within the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// One-based line number.
    pub line: usize,
    /// One-based column, counted in characters rather than bytes.
    pub column: usize,
}

/// Convert a zero-based byte offset into a one-based line and column.
///
/// An offset past the end of the source clamps to the end, which is what an
/// end-of-input diagnostic needs.
#[must_use]
pub fn position(source: &str, offset: usize) -> Position {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |index| index + 1);
    Position {
        line,
        column: source[start..offset].chars().count() + 1,
    }
}

/// Shorten a value's printed form for a line a person reads; the JSON form
/// of a diagnostic always carries the whole value.
#[must_use]
pub fn shorten(text: &str) -> String {
    const LIMIT: usize = 120;
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let kept: String = text.chars().take(LIMIT).collect();
    format!("{kept}…")
}

/// Render a diagnostic as `DiagnosticText`: the heading, the source line and
/// a caret, then its labels, help, expected and found, element and trace.
///
/// `source` is the text the diagnostic's span points into, absent for a
/// diagnostic about a document that was not read; `origin` names it when the
/// diagnostic does not.
#[must_use]
pub fn diagnostic_text(diagnostic: &Diagnostic, source: Option<&str>, origin: &str) -> String {
    // A cell of a document is written `path#cell`: the location is the
    // document's, and the cell is named on a line of its own.
    let (origin, cell) = match diagnostic
        .origin
        .as_deref()
        .unwrap_or(origin)
        .split_once('#')
    {
        Some((file, cell)) => (file, Some(cell)),
        None => (diagnostic.origin.as_deref().unwrap_or(origin), None),
    };
    let heading = format!(
        "{} {}[{}]: {}",
        diagnostic.phase.label(),
        diagnostic.severity.label(),
        diagnostic.kind.label(),
        diagnostic.message
    );
    let Some(source) = source else {
        let mut rendered = format!("{heading}\n --> {origin}");
        for line in notes(diagnostic, None, origin, cell, " ") {
            rendered.push_str(&format!("\n{line}"));
        }
        return rendered;
    };
    let Position { line, column } = position(source, diagnostic.span.start);
    let text = source.lines().nth(line - 1).unwrap_or("");

    // An empty span means end of input; still show one caret, so the
    // diagnostic never points at nothing.
    let width = source
        .get(diagnostic.span.start..diagnostic.span.end.min(source.len()))
        .map_or(1, |slice| slice.chars().count().max(1));
    let gutter = line.to_string();
    let pad = " ".repeat(gutter.len());
    let mut rendered = format!(
        "{heading}\n\
         {pad}--> {origin}:{line}:{column}\n\
         {pad} |\n\
         {gutter} | {text}\n\
         {pad} | {caret_pad}{caret}",
        caret_pad = " ".repeat(column - 1),
        caret = "^".repeat(width),
    );
    for note in notes(diagnostic, Some(source), origin, cell, &pad) {
        rendered.push_str(&format!("\n{note}"));
    }
    rendered
}

/// The `= …` lines under a diagnostic's caret, one per field that applies.
fn notes(
    diagnostic: &Diagnostic,
    source: Option<&str>,
    origin: &str,
    cell: Option<&str>,
    pad: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(cell) = cell {
        lines.push(format!("{pad} = cell: {cell}"));
    }
    for label in &diagnostic.context.labels {
        let at = label.origin.as_deref().unwrap_or(origin);
        let place = match (source, &label.origin) {
            (Some(source), None) => {
                let Position { line, column } = position(source, label.span.start);
                format!("{at}:{line}:{column}")
            }
            _ => at.to_owned(),
        };
        lines.push(format!("{pad} = note: {}, at {place}", label.text));
    }
    for help in &diagnostic.context.help {
        lines.push(format!("{pad} = help: {help}"));
    }
    if let Some(expected) = &diagnostic.context.expected {
        lines.push(format!("{pad} = expected: {expected}"));
    }
    if let Some(found) = &diagnostic.context.found {
        lines.push(format!("{pad} = found: {}", shorten(&found.to_string())));
    }
    if let Some(element) = &diagnostic.context.element {
        let of = element
            .of
            .map_or_else(String::new, |of| format!(" of {of}"));
        lines.push(format!(
            "{pad} = element: {}, position {}{of} in `{}`",
            shorten(&element.value.to_string()),
            element.position,
            element.step
        ));
    }
    if let Some(source) = &diagnostic.context.source {
        lines.push(format!("{pad} = source: {}", source.path));
    }
    if !diagnostic.context.bindings.is_empty() {
        let bound: Vec<String> = diagnostic
            .context
            .bindings
            .iter()
            .map(|(name, value)| format!("{name} = {}", shorten(&value.to_string())))
            .collect();
        lines.push(format!("{pad} = bindings: {}", bound.join(", ")));
    }
    if !diagnostic.context.trace.is_empty() {
        let frames: Vec<String> = diagnostic
            .context
            .trace
            .iter()
            .map(|frame| match frame.position {
                Some(position) => format!("{} (step {position})", frame.name),
                None => format!("{} (call)", frame.name),
            })
            .collect();
        lines.push(format!("{pad} = trace: {}", frames.join(" → ")));
    }
    lines
}

/// The count per severity a report ends with, when it holds more than one
/// diagnostic.
#[must_use]
pub fn summary(diagnostics: &[Diagnostic]) -> Option<String> {
    use crate::diagnostics::{Severity, count};
    (diagnostics.len() > 1).then(|| {
        let mut rendered = format!(
            "{} diagnostics: {} failure, {} error, {} warning",
            diagnostics.len(),
            count(diagnostics, Severity::Failure),
            count(diagnostics, Severity::Error),
            count(diagnostics, Severity::Warning),
        );
        for severity in [Severity::Info, Severity::Debug] {
            let found = count(diagnostics, severity);
            if found > 0 {
                rendered.push_str(&format!(", {found} {}", severity.label()));
            }
        }
        rendered
    })
}

/// Render the channel's contents as a report, `Seq<Diagnostic>` presented:
/// each diagnostic in `DiagnosticText`, then the summary.
#[must_use]
pub fn report(diagnostics: &[Diagnostic], source: Option<&str>, origin: &str) -> String {
    let mut rendered = String::new();
    for diagnostic in diagnostics {
        rendered.push_str(&diagnostic_text(diagnostic, source, origin));
        rendered.push_str("\n\n");
    }
    if let Some(summary) = summary(diagnostics) {
        rendered.push_str(&summary);
        rendered.push('\n');
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_characters_not_bytes() {
        assert_eq!(position("a\nbc", 3), Position { line: 2, column: 2 });
        assert_eq!(position("\u{1f600}x", 4), Position { line: 1, column: 2 });
        // Past the end clamps, which is what end of input needs.
        assert_eq!(position("ab", 99), Position { line: 1, column: 3 });
    }

    #[test]
    fn a_diagnostic_points_at_its_line_with_a_caret_under_the_span() {
        let rendered = diagnostic_text(
            &Diagnostic::typing(6..10, "no"),
            Some("1 +\n  true"),
            "<expression>",
        );
        assert!(
            rendered.contains("check error[type-mismatch]: no"),
            "{rendered}"
        );
        assert!(rendered.contains("<expression>:2:3"), "{rendered}");
        assert!(rendered.ends_with("^^^^"), "{rendered}");
    }

    #[test]
    fn help_follows_the_caret() {
        let rendered = diagnostic_text(
            &Diagnostic::name(0..4, "`mapp` is not a pipeline step")
                .with_help("did you mean `map`?"),
            Some("mapp"),
            "<expression>",
        );
        assert!(
            rendered.ends_with("\n  = help: did you mean `map`?"),
            "{rendered}"
        );
    }
}
