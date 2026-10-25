//! Doc strings: a block of `///` lines opened after an empty line documents
//! the declarations that begin in the paragraph below it, up to the next
//! empty line (HQL-0042).
//!
//! The lexer skips `///` as it skips `//`; this module finds the blocks in
//! the source text, holds each to where it may stand, and attaches it to the
//! declarations the parser built. The text of a doc string is CommonMark,
//! kept as written, and its citations are `[[/path]]` references to the
//! published site.

use crate::ast::{Program, Stmt};
use crate::diagnostics::Diagnostic;
use std::ops::Range;
use std::sync::Arc;

/// Where the published documentation lives; a citation's URL is this, the
/// binary's version, and the page's path.
pub const SITE: &str = "https://hql-lang.org/";

/// One doc string: its text, the doc lines joined as written, and the bytes
/// of those lines.
#[derive(Debug, PartialEq, Eq)]
pub struct DocString {
    pub text: String,
    pub span: Range<usize>,
}

/// The doc strings of one parsed source, by the declarations they document.
/// The declarations of one paragraph hold the same [`DocString`].
#[derive(Debug, Default)]
pub(crate) struct Docs {
    /// Top-level declarations, by where their statement begins.
    statements: Vec<(usize, Arc<DocString>)>,
    /// Members of a `trait` or an `impl`, by where the member begins.
    members: Vec<(usize, Arc<DocString>)>,
}

impl Docs {
    /// The doc string of the top-level declaration beginning at `start`.
    pub(crate) fn statement(&self, start: usize) -> Option<&Arc<DocString>> {
        self.statements
            .iter()
            .find(|(at, _)| *at == start)
            .map(|(_, doc)| doc)
    }

    /// The doc string of the trait or impl member beginning at `start`.
    pub(crate) fn member(&self, start: usize) -> Option<&Arc<DocString>> {
        self.members
            .iter()
            .find(|(at, _)| *at == start)
            .map(|(_, doc)| doc)
    }
}

/// Whether a line is a doc line: `///` first, and no fourth slash.
fn is_doc_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("///") && !trimmed.starts_with("////")
}

/// The text of a doc line: what follows `///`, less one leading space.
fn doc_text(line: &str) -> &str {
    let after = &line.trim_start()[3..];
    after.strip_prefix(' ').unwrap_or(after)
}

/// The statement's first byte: where its name or keyword is written.
pub(crate) fn statement_start(statement: &Stmt) -> usize {
    match statement {
        Stmt::Bind { name_span, .. } => name_span.start,
        Stmt::Import { span, .. } | Stmt::Module { span, .. } => span.start,
        Stmt::Type(declaration) => declaration.span.start,
        Stmt::Fn(declaration) => declaration.span.start,
        Stmt::External(declaration) => declaration.span.start,
        Stmt::Alias(declaration) => declaration.span.start,
        Stmt::Trait(declaration) => declaration.span.start,
        Stmt::Impl(declaration) => declaration.span.start,
        Stmt::Expr(expression) => expression.span.start,
    }
}

/// Whether a statement is a declaration a doc string may document.
fn documentable(statement: &Stmt) -> bool {
    !matches!(
        statement,
        Stmt::Bind { .. } | Stmt::Import { .. } | Stmt::Expr(_)
    )
}

/// One line of the source: its bytes, without the line break.
struct Line<'a> {
    text: &'a str,
    start: usize,
}

impl Line<'_> {
    fn blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    fn comment(&self) -> bool {
        self.text.trim_start().starts_with("//") && !is_doc_line(self.text)
    }

    fn end(&self) -> usize {
        self.start + self.text.len()
    }
}

fn lines(source: &str) -> Vec<Line<'_>> {
    let mut start = 0;
    source
        .split('\n')
        .map(|text| {
            let line = Line { text, start };
            start += text.len() + 1;
            line
        })
        .collect()
}

/// Find the doc blocks of a parsed source and attach each to the
/// declarations that begin in its paragraph.
///
/// # Errors
///
/// A doc block that does not open after an empty line, one with no
/// declaration directly below it, and a paragraph holding a binding, an
/// expression or an `import`.
pub(crate) fn attach(source: &str, program: &Program) -> Result<Docs, Diagnostic> {
    let lines = lines(source);
    let line_of = |offset: usize| {
        lines
            .partition_point(|line| line.start <= offset)
            .saturating_sub(1)
    };
    let starts: Vec<(usize, &Stmt)> = program
        .statements
        .iter()
        .map(|statement| (statement_start(statement), statement))
        .collect();
    let members: Vec<usize> = program
        .statements
        .iter()
        .flat_map(|statement| match statement {
            Stmt::Trait(declaration) => declaration.methods.iter().collect::<Vec<_>>(),
            Stmt::Impl(declaration) => declaration.methods.iter().collect(),
            _ => Vec::new(),
        })
        .map(|method| method.span.start)
        .collect();
    let mut docs = Docs::default();
    let mut at = 0;
    while at < lines.len() {
        if !is_doc_line(lines[at].text) {
            at += 1;
            continue;
        }
        let first = at;
        while at < lines.len() && is_doc_line(lines[at].text) {
            at += 1;
        }
        let span = lines[first].start + lines[first].text.len()
            - lines[first].text.trim_start().len()..lines[at - 1].end();
        // Above the block: an empty line, the start of the input, or the
        // line opening a body, past any `//` lines.
        let mut above = first;
        while above > 0 && lines[above - 1].comment() {
            above -= 1;
        }
        if above > 0
            && !lines[above - 1].blank()
            && !lines[above - 1].text.trim_end().ends_with('{')
        {
            return Err(Diagnostic::syntax(
                span,
                "a doc string opens after an empty line",
            ));
        }
        let detached = || {
            Diagnostic::syntax(
                span.clone(),
                "a doc string must be directly above a declaration",
            )
        };
        if at == lines.len() || lines[at].blank() || lines[at].comment() {
            return Err(detached());
        }
        let mut last = at;
        while last + 1 < lines.len()
            && !lines[last + 1].blank()
            && !lines[last + 1].text.trim_start().starts_with('}')
            && !is_doc_line(lines[last + 1].text)
        {
            last += 1;
        }
        let paragraph = at..=last;
        let doc = Arc::new(DocString {
            text: lines[first..at]
                .iter()
                .map(|line| doc_text(line.text))
                .collect::<Vec<_>>()
                .join("\n"),
            span: span.clone(),
        });
        let opens_statement = starts.iter().any(|(start, _)| line_of(*start) == at);
        if opens_statement {
            let opener = starts
                .iter()
                .find(|(start, _)| line_of(*start) == at)
                .map(|(_, statement)| statement);
            if opener.is_some_and(|statement| !documentable(statement)) {
                return Err(detached());
            }
            for (start, statement) in &starts {
                if !paragraph.contains(&line_of(*start)) {
                    continue;
                }
                if !documentable(statement) {
                    return Err(Diagnostic::syntax(
                        *start..lines[line_of(*start)].end(),
                        "a documented paragraph holds declarations only",
                    ));
                }
                docs.statements.push((*start, Arc::clone(&doc)));
            }
        } else if members.iter().any(|start| line_of(*start) == at) {
            for start in &members {
                if paragraph.contains(&line_of(*start)) {
                    docs.members.push((*start, Arc::clone(&doc)));
                }
            }
        } else {
            return Err(detached());
        }
        at = last + 1;
    }
    Ok(docs)
}

/// The first sentence of a doc string: up to and including the first `.`,
/// `?` or `!` followed by whitespace or the end, outside a code span, with
/// its line breaks as spaces. Citations print as [`render`] has them in a
/// sentence: the display text, or the URL in angle brackets.
#[must_use]
pub fn first_sentence(text: &str, version: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut code = false;
    let mut end = chars.len();
    for (at, c) in chars.iter().enumerate() {
        match c {
            '`' => code = !code,
            '.' | '?' | '!'
                if !code && chars.get(at + 1).is_none_or(|next| next.is_whitespace()) =>
            {
                end = at + 1;
                break;
            }
            _ => {}
        }
    }
    let sentence: String = chars[..end].iter().collect();
    let joined = sentence
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    cite(&joined, version, true)
}

/// A doc string's text as `help` prints it: each citation `[[/path|text]]`
/// as `text <URL>`, and `[[/path]]` as `<URL>`.
#[must_use]
pub fn render(text: &str, version: &str) -> String {
    cite(text, version, false)
}

/// Whether a position of a text is inside a code span or block: code is
/// shown as written and cites nothing.
pub(crate) fn in_code(text: &str, at: usize) -> bool {
    text[..at].matches('`').count() % 2 == 1
}

/// Replace the absolute citations of a text by their URLs; in a sentence,
/// one with display text prints only that text.
fn cite(text: &str, version: &str, sentence: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let Some(close) = rest[open..].find("]]") else {
            break;
        };
        if in_code(text, text.len() - rest.len() + open) {
            out.push_str(&rest[..open + 2]);
            rest = &rest[open + 2..];
            continue;
        }
        let inner = &rest[open + 2..open + close];
        out.push_str(&rest[..open]);
        let (target, display) = match inner.split_once('|') {
            Some((target, display)) => (target, Some(display)),
            None => (inner, None),
        };
        match (citation_url(target, version), display) {
            (Some(_), Some(display)) if sentence => out.push_str(display),
            (Some(url), Some(display)) => out.push_str(&format!("{display} <{url}>")),
            (Some(url), None) => out.push_str(&format!("<{url}>")),
            (None, _) => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &rest[open + close + 2..];
    }
    out.push_str(rest);
    out
}

/// The URL an absolute citation names under a version: the page's path from
/// the site's root, a folder's `index` dropped, ending in `/`, then the
/// fragment. A citation that is not absolute has none.
#[must_use]
pub fn citation_url(target: &str, version: &str) -> Option<String> {
    let path = target.strip_prefix('/')?;
    let (path, fragment) = match path.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (path, None),
    };
    let path = path.strip_suffix("index").unwrap_or(path);
    let path = path.trim_end_matches('/');
    let mut url = format!("{SITE}v{version}/");
    if !path.is_empty() {
        url.push_str(path);
        url.push('/');
    }
    if let Some(fragment) = fragment {
        url.push('#');
        url.push_str(fragment);
    }
    Some(url)
}

/// The citations a doc string writes, as written between `[[` and `]]`,
/// without display text.
#[must_use]
pub fn citations(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let Some(close) = rest[open..].find("]]") else {
            break;
        };
        if in_code(text, text.len() - rest.len() + open) {
            rest = &rest[open + 2..];
            continue;
        }
        let inner = &rest[open + 2..open + close];
        found.push(inner.split_once('|').map_or(inner, |(target, _)| target));
        rest = &rest[open + close + 2..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(source: &str) -> Result<(Program, Docs), Diagnostic> {
        let program = crate::parser::parse(source)?;
        let docs = attach(source, &program)?;
        Ok((program, docs))
    }

    fn message(source: &str) -> String {
        crate::parser::parse(source).unwrap_err().message()
    }

    #[test]
    fn a_block_documents_the_declarations_of_its_paragraph() {
        let source = "/// Twice `n`.\n///\n/// Exactly.\nfn double(n: Int) : Int = n * 2\nfn triple(n: Int) : Int = n * 3\n\nfn four() : Int = 4\n\n1";
        let (program, docs) = parsed(source).unwrap();
        let doc = |at: usize| docs.statement(statement_start(&program.statements[at]));
        assert_eq!(doc(0).unwrap().text, "Twice `n`.\n\nExactly.");
        assert!(Arc::ptr_eq(doc(0).unwrap(), doc(1).unwrap()));
        assert!(doc(2).is_none());
        assert_eq!(
            &source[doc(0).unwrap().span.clone()],
            "/// Twice `n`.\n///\n/// Exactly."
        );
    }

    #[test]
    fn four_slashes_are_a_comment_and_one_space_is_stripped() {
        let source = "//// a rule\n///  two spaces\nfn f() : Int = 1\n\nf()";
        let (program, docs) = parsed(source).unwrap();
        let doc = docs
            .statement(statement_start(&program.statements[0]))
            .unwrap();
        assert_eq!(doc.text, " two spaces");
    }

    #[test]
    fn comments_may_stand_above_a_block() {
        let source = "x = 1\n\n// HQL-0042\n/// One.\nfn one() : Int = 1\n\none()";
        assert!(parsed(source).is_ok());
    }

    #[test]
    fn a_member_takes_the_block_inside_its_body() {
        let source =
            "/// Shapes.\ntrait Shape {\n    /// The area.\n    fn area(self) : Float\n}\n\n1";
        let (program, docs) = parsed(source).unwrap();
        let Stmt::Trait(shape) = &program.statements[0] else {
            panic!()
        };
        assert_eq!(docs.statement(shape.span.start).unwrap().text, "Shapes.");
        assert_eq!(
            docs.member(shape.methods[0].span.start).unwrap().text,
            "The area."
        );
    }

    #[test]
    fn misplaced_blocks_are_refused() {
        assert_eq!(
            message("x = 1\n/// One.\nfn one() : Int = 1\n\none()"),
            "a doc string opens after an empty line"
        );
        assert_eq!(
            message("/// One.\n\nfn one() : Int = 1\n\none()"),
            "a doc string must be directly above a declaration"
        );
        assert_eq!(
            message("1\n\n/// One."),
            "a doc string must be directly above a declaration"
        );
        assert_eq!(
            message("/// One.\nx = 1\n\nx"),
            "a doc string must be directly above a declaration"
        );
        assert_eq!(
            message("/// One.\nfn one() : Int = 1\nx = 1\n\nx"),
            "a documented paragraph holds declarations only"
        );
        assert_eq!(
            message("fn one() : Int = 1 /// One.\n\none()"),
            "a doc string stands on its own lines, directly above a declaration"
        );
    }

    #[test]
    fn the_first_sentence_ends_outside_code() {
        assert_eq!(
            first_sentence("Calls `a.b` once. Then more.", "1.0.0"),
            "Calls `a.b` once."
        );
        assert_eq!(first_sentence("No mark at\nall", "1.0.0"), "No mark at all");
        assert_eq!(
            first_sentence("See [[/learn/x|X]]. More.", "1.0.0"),
            "See X."
        );
    }

    #[test]
    fn citations_print_as_versioned_urls() {
        assert_eq!(
            render(
                "See [[/learn/02-data-types/decimal#division-and-rounding|Division]].",
                "0.5.0"
            ),
            "See Division <https://hql-lang.org/v0.5.0/learn/02-data-types/decimal/#division-and-rounding>."
        );
        assert_eq!(
            render("[[/reference/std/seq/index]]", "0.5.0"),
            "<https://hql-lang.org/v0.5.0/reference/std/seq/>"
        );
        assert_eq!(render("[[relative]]", "0.5.0"), "[[relative]]");
        assert_eq!(citations("[[/a|b]] and [[c]]"), vec!["/a", "c"]);
        assert!(citations("`[[1, 2], [3]]`").is_empty());
    }
}
