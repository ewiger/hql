//! Human-readable reports: origin, line and column, and a caret.
//!
//! The byte ranges a report carries are the machine-readable form and stay
//! that way; rendering them for a person is a separate, testable step rather
//! than something only the binary can do.

use crate::reporting::Report;

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
/// end-of-input report needs.
#[must_use]
pub fn position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |index| index + 1);
    Position {
        line,
        column: source[start..offset].chars().count() + 1,
    }
}

/// Render a report as a multi-line message pointing into the source.
///
/// `origin` labels the source: a path, or something like `<expression>` when
/// the program did not come from a file.
#[must_use]
pub fn report(report: &Report, source: &str, origin: &str) -> String {
    let Position { line, column } = position(source, report.span.start);
    let text = source.lines().nth(line - 1).unwrap_or("");

    // An empty span means end of input; still show one caret, so the report
    // never points at nothing.
    let width = source
        .get(report.span.start..report.span.end.min(source.len()))
        .map_or(1, |slice| slice.chars().count().max(1));
    let gutter = line.to_string();
    let pad = " ".repeat(gutter.len());

    // A capture is headed by its label, the way a coded entry is headed by
    // its code; it has no stage worth naming, the program wrote it.
    let heading = match (&report.code, &report.label) {
        (Some(code), label) if code == "debug" => match label {
            Some(label) => format!("{}[{label}]", report.severity.label()),
            None => report.severity.label().to_owned(),
        },
        _ => format!("{} {}", report.stage, report.severity.label()),
    };
    let mut rendered = format!(
        "{heading}: {message}\n\
         {pad}--> {origin}:{line}:{column}\n\
         {pad} |\n\
         {gutter} | {text}\n\
         {pad} | {caret_pad}{caret}",
        message = report.message,
        caret_pad = " ".repeat(column - 1),
        caret = "^".repeat(width),
    );
    if let Some(shown) = &report.shown {
        let value = crate::reporting::shorten(shown);
        if value != report.message {
            rendered.push_str(&format!("\n{pad} = value: {value}"));
        }
    }
    if !report.trace.is_empty() {
        let steps: Vec<String> = report
            .trace
            .iter()
            .map(|step| match &step.element {
                Some(element) => {
                    format!("{} (step {}), element {element}", step.step, step.position)
                }
                None => format!("{} (step {})", step.step, step.position),
            })
            .collect();
        rendered.push_str(&format!("\n{pad} = trace: {}", steps.join(" → ")));
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;

    #[test]
    fn positions_count_characters_not_bytes() {
        assert_eq!(position("a\nbc", 3), Position { line: 2, column: 2 });
        assert_eq!(position("\u{1f600}x", 4), Position { line: 1, column: 2 });
        // Past the end clamps, which is what end of input needs.
        assert_eq!(position("ab", 99), Position { line: 1, column: 3 });
    }

    #[test]
    fn a_report_points_at_its_line_with_a_caret_under_the_span() {
        let rendered = report(
            &Report::of(&Diagnostic::typing(6..10, "no")),
            "1 +\n  true",
            "<expression>",
        );
        assert!(rendered.contains("type error: no"), "{rendered}");
        assert!(rendered.contains("<expression>:2:3"), "{rendered}");
        assert!(rendered.ends_with("^^^^"), "{rendered}");
    }
}
