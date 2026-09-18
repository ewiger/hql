//! Documents and the cards built from them.
//!
//! A document is whatever a source supplies: a name, a path, a header and a
//! body. A card adds the knowledge layer, and card-ness is a knowledge notion
//! rather than a file format — `.md` and `.hmd` both become cards here.

use crate::data::{self, Data};
use std::path::{Path, PathBuf};

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
    #[must_use]
    pub fn parse(name: &str, path: &Path, format: Format, source: &str) -> Self {
        let (authored, body) = split_header(source);
        let mut header = data::parse_header(authored);
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

        Self {
            name: name.to_owned(),
            path: path.to_owned(),
            format,
            header,
            body: body.to_owned(),
            references: references(body),
        }
    }

    /// The document's title, which falls back to its name.
    #[must_use]
    pub fn title(&self) -> &str {
        self.header
            .path("title")
            .and_then(Data::as_str)
            .unwrap_or(&self.name)
    }

    /// The words an index sees: title, header strings and body text.
    #[must_use]
    pub fn text(&self) -> String {
        let mut parts = vec![self.title().to_owned()];
        self.header.words(&mut parts);
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
fn split_header(source: &str) -> (&str, &str) {
    let rest = source.strip_prefix("---\n").or_else(|| {
        source
            .strip_prefix("---\r\n")
            .or_else(|| source.strip_prefix("---").filter(|r| r.starts_with('\n')))
    });
    let Some(rest) = rest else {
        return ("", source);
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (&rest[..offset], &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    ("", source)
}

/// The first ATX heading in a body, which is a document's title when the
/// header does not state one.
fn heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(|text| text.trim().to_owned())
}

/// Collect `[[target]]` references and the `{..}` annotations that follow them.
fn references(body: &str) -> Vec<Reference> {
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
            body[probe..].find('}').map_or(Data::map(), |close| {
                let inner = &body[probe + 1..probe + close];
                offset = probe + close + 1;
                data::parse_header(&inner.replace(',', "\n"))
            })
        } else {
            Data::map()
        };
        found.push(Reference { target, data });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(source: &str) -> Document {
        Document::parse("alice", Path::new("alice.hmd"), Format::Hmd, source)
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
