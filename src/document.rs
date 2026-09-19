//! Documents and the cards built from them.
//!
//! A document is whatever a source supplies: a name, a path, a header and a
//! body. A card adds the knowledge layer, and card-ness is a knowledge notion
//! rather than a file format — `.md` and `.hmd` both become cards here.

use crate::data::{self, Data};
use std::path::{Path, PathBuf};

/// Parsed documents and source diagnostics before knowledge assembly.
pub(crate) struct DocumentStore {
    pub documents: Vec<Document>,
    pub warnings: Vec<String>,
}

impl DocumentStore {
    pub fn from_source(mut snapshot: crate::sources::Snapshot) -> Self {
        snapshot
            .documents
            .sort_by(|left, right| left.path.cmp(&right.path));
        let stem = |path: &Path| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned()
        };
        let mut counts = std::collections::BTreeMap::new();
        for document in &snapshot.documents {
            *counts.entry(stem(&document.path)).or_insert(0) += 1;
        }
        let mut store = Self {
            documents: Vec::new(),
            warnings: Vec::new(),
        };
        for document in snapshot.documents {
            let path = snapshot.root.join(&document.path);
            let text = match document.text {
                Ok(text) => text,
                Err(error) => {
                    store.warnings.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };
            let mut name = stem(&document.path);
            if counts.get(&name).copied().unwrap_or_default() > 1 {
                name = document
                    .path
                    .with_extension("")
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
            }
            match Document::parse(&name, &path, document.format, &text) {
                Ok(document) => store.documents.push(document),
                Err(error) => store
                    .warnings
                    .push(format!("{}: {error}; document skipped", path.display())),
            }
        }
        store
    }
}

/// A document boundary or YAML tree that cannot be loaded faithfully.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid front matter.
    #[error("header: {0}")]
    Header(#[from] data::Error),
    /// A leading front matter fence was not closed.
    #[error("unterminated front matter")]
    UnclosedHeader,
    /// A reference's YAML annotation was invalid.
    #[error("reference annotation at body byte {offset}: {source}")]
    Annotation { offset: usize, source: data::Error },
    /// A reference annotation was not closed.
    #[error("unterminated reference annotation at body byte {0}")]
    UnclosedAnnotation(usize),
}

/// The dialect a document was written in, as `header.format` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// HyperMarkDown.
    Hmd,
    /// Plain Markdown.
    Markdown,
}

impl Format {
    /// The dialect implied by a file extension, when the source is a file.
    #[must_use]
    pub fn of(path: &Path) -> Option<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("hmd") => Some(Self::Hmd),
            Some("md" | "markdown") => Some(Self::Markdown),
            _ => None,
        }
    }

    /// The dialect's name, as `header.format` reports it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Hmd => "Hmd",
            Self::Markdown => "Markdown",
        }
    }
}

/// A reference written in a body: `[[target]]`, with its optional annotation.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The name the reference points at, which need not resolve.
    pub target: String,
    /// The annotation written in braces after it.
    pub data: Data,
}

/// A document: a header, a body, and the references the body writes.
#[derive(Debug, Clone)]
pub struct Document {
    /// The document's name in its namespace — the file stem for a file.
    pub name: String,
    /// Where it came from.
    pub path: PathBuf,
    /// Its dialect.
    pub format: Format,
    /// Everything known about the document, derived and authored.
    pub header: Data,
    /// The prose after the header.
    pub body: String,
    /// The references the body writes, in the order they appear.
    pub references: Vec<Reference>,
}

impl Document {
    /// Build a document from source text, splitting a `---` header if present.
    ///
    /// # Errors
    /// Rejects malformed front matter and reference annotations.
    pub fn parse(name: &str, path: &Path, format: Format, source: &str) -> Result<Self, Error> {
        let (authored, body) = split_header(source)?;
        let mut header = data::parse_header(authored)?;
        let title = header
            .path("title")
            .and_then(Data::as_str)
            .map(str::to_owned)
            .or_else(|| heading(body));

        // Derived entries own their paths: nothing authored may contradict
        // where a document is or what it is called.
        header.insert_path("name", Data::Str(name.to_owned()));
        header.insert_path("path", Data::Str(path.display().to_string()));
        header.insert_path("format", Data::Str(format.name().to_owned()));
        if let Some(title) = title {
            header.insert_path("title", Data::Str(title));
        }

        Ok(Self {
            name: name.to_owned(),
            path: path.to_owned(),
            format,
            header,
            body: body.to_owned(),
            references: references(body)?,
        })
    }

    /// The document's title, which falls back to its name.
    #[must_use]
    pub fn title(&self) -> &str {
        self.header
            .path("title")
            .and_then(Data::as_str)
            .unwrap_or(&self.name)
    }

    /// The words an index sees: the title, the authored header's strings in
    /// key order, and the body.
    ///
    /// The derived entries are left out. `name`, `path` and `format` say where
    /// a document sits and what it is called, not what it says, and a card
    /// that moves between directories must not thereby change its score. The
    /// title is written once, at the front, rather than again in key order.
    #[must_use]
    pub fn text(&self) -> String {
        let mut parts = vec![self.title().to_owned()];
        if let Data::Map(entries) = &self.header {
            for (key, value) in entries {
                if matches!(key.as_str(), "name" | "path" | "format" | "title") {
                    continue;
                }
                value.words(&mut parts);
            }
        }
        parts.push(self.body.clone());
        parts.join(" ")
    }
}

/// Which kind of knowledge a card represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The ordinary case: the card is a concept and occupies a node position.
    Concept,
    /// The exception: the card represents a relation, which is an edge.
    Relation,
}

/// A document that gives a piece of knowledge a physical representation.
#[derive(Debug, Clone)]
pub struct Card {
    /// The document underneath.
    pub document: Document,
    /// The assembled knowledge layer.
    pub metadata: Data,
    /// Which kind the card is, after the declared kind has been checked.
    pub kind: Kind,
}

impl Card {
    /// The card's name, which is also its canonical ordering key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.document.name
    }

    /// The relation endpoints a relation card represents.
    #[must_use]
    pub fn endpoints(&self) -> Option<(String, String)> {
        let source = self.metadata.path("knowledge.relation.source")?.as_str()?;
        let target = self.metadata.path("knowledge.relation.target")?.as_str()?;
        Some((source.to_owned(), target.to_owned()))
    }
}

/// Split a leading `---` fenced header from the body.
fn split_header(source: &str) -> Result<(&str, &str), Error> {
    let rest = source.strip_prefix("---\n").or_else(|| {
        source
            .strip_prefix("---\r\n")
            .or_else(|| source.strip_prefix("---").filter(|r| r.starts_with('\n')))
    });
    let Some(rest) = rest else {
        return Ok(("", source));
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Ok((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    Err(Error::UnclosedHeader)
}

/// The first ATX heading in a body, which is a document's title when the
/// header does not state one.
fn heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(|text| text.trim().to_owned())
}

/// Collect `[[target]]` references and the `{..}` annotations that follow them.
fn references(body: &str) -> Result<Vec<Reference>, Error> {
    let mut found = Vec::new();
    let bytes = body.as_bytes();
    let mut offset = 0;
    while let Some(start) = body[offset..].find("[[") {
        let open = offset + start + 2;
        let Some(length) = body[open..].find("]]") else {
            break;
        };
        let target = body[open..open + length].trim().to_owned();
        offset = open + length + 2;
        if target.is_empty() {
            continue;
        }
        // An annotation binds to the reference only when it follows directly,
        // separated by nothing but spaces on the same line.
        let mut probe = offset;
        while bytes.get(probe) == Some(&b' ') {
            probe += 1;
        }
        let data = if bytes.get(probe) == Some(&b'{') {
            let end = annotation_end(&body[probe..]).ok_or(Error::UnclosedAnnotation(probe))?;
            offset = probe + end;
            data::parse_header(&body[probe..offset]).map_err(|source| Error::Annotation {
                offset: probe,
                source,
            })?
        } else {
            Data::map()
        };
        found.push(Reference { target, data });
    }
    Ok(found)
}

/// Locate the flow mapping boundary; YAML itself parses its contents.
fn annotation_end(source: &str) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut previous = None;
    let mut chars = source.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && delimiter == '"' {
                escaped = true;
            } else if ch == delimiter {
                if delimiter == '\'' && chars.peek().is_some_and(|(_, next)| *next == '\'') {
                    chars.next();
                } else {
                    quote = None;
                    previous = Some(ch);
                }
            }
            continue;
        }
        match ch {
            '\'' | '"' if matches!(previous, Some('{' | '[' | ',' | ':')) => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset + 1);
                }
            }
            _ => {}
        }
        if !ch.is_whitespace() {
            previous = Some(ch);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(source: &str) -> Document {
        Document::parse("alice", Path::new("alice.hmd"), Format::Hmd, source).unwrap()
    }

    #[test]
    fn splits_a_header_and_derives_entries() {
        let doc = document("---\ntitle: Alice\n---\n# Ignored\n\nbody\n");
        assert_eq!(doc.title(), "Alice");
        assert_eq!(doc.body.trim_start(), "# Ignored\n\nbody\n".trim_start());
        assert_eq!(
            doc.header.path("format").and_then(Data::as_str),
            Some("Hmd")
        );
        assert_eq!(
            doc.header.path("name").and_then(Data::as_str),
            Some("alice")
        );
    }

    #[test]
    fn a_heading_supplies_the_title_when_the_header_does_not() {
        assert_eq!(document("# Bearer tokens\n\ntext").title(), "Bearer tokens");
        assert_eq!(document("no heading").title(), "alice");
    }

    #[test]
    fn collects_references_with_and_without_annotations() {
        let doc = document("see [[bob]] {relation: parent} and [[carol]].\n");
        let names: Vec<&str> = doc
            .references
            .iter()
            .map(|reference| reference.target.as_str())
            .collect();
        assert_eq!(names, ["bob", "carol"]);
        assert_eq!(
            doc.references[0]
                .data
                .path("relation")
                .and_then(Data::as_str),
            Some("parent")
        );
        assert_eq!(doc.references[1].data, Data::map());
    }

    #[test]
    fn a_document_without_a_header_keeps_its_whole_body() {
        let doc = document("just text\n");
        assert_eq!(doc.body, "just text\n");
    }
}
