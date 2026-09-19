//! Source assembly and the YAML-to-Data boundary preserve authored meaning.

use hql::data::{Data, parse_header};
use hql::document::{Document, Format};
use hql::sources::{Snapshot, SourceDocument, VaultSource};
use std::path::Path;

#[test]
fn yaml_preserves_quotes_blocks_nested_flows_and_aliases() {
    let data = parse_header("tags: [\"foo,bar\", baz]\nquoted: \"line\\nnext\"\nblock: |\n  first\n  second\nbase: &base {nested: [true, {value: 0x10}]}\ncopy: *base\nmerged: {<<: *base, extra: 1}\nnumber: 1e3\nnothing: null\n").unwrap();
    assert_eq!(
        data.path("tags"),
        Some(&Data::List(vec![
            Data::Str("foo,bar".into()),
            Data::Str("baz".into())
        ]))
    );
    assert_eq!(
        data.path("quoted").and_then(Data::as_str),
        Some("line\nnext")
    );
    assert_eq!(
        data.path("block").and_then(Data::as_str),
        Some("first\nsecond\n")
    );
    assert_eq!(data.path("base"), data.path("copy"));
    assert_eq!(data.path("base.nested"), data.path("merged.nested"));
    assert_eq!(data.path("number"), Some(&Data::Float(1000.0)));
    assert_eq!(data.path("nothing"), Some(&Data::Empty));
}

#[test]
fn invalid_or_unrepresentable_yaml_is_reported() {
    for source in [
        "tags: [broken",
        "value: .inf",
        "value: .nan",
        "value: 9223372036854775808",
        "value: !custom text",
        "1: value",
        "? [a, b]\n: value",
        "a: 1\na: 2",
        "[one, two]",
        "plain scalar",
        "first: 1\n---\nsecond: 2",
    ] {
        assert!(parse_header(source).is_err(), "{source}");
    }
}

#[test]
fn reference_annotations_use_yaml_without_comma_splitting() {
    for annotation in ["{note: 3 o'clock}", "{note: 'it''s } nested', other: true}"] {
        let doc = Document::parse(
            "note",
            Path::new("note.hmd"),
            Format::Hmd,
            &format!("[[bird]] {annotation} and [[tree]]"),
        )
        .unwrap();
        assert_eq!(doc.references.len(), 2);
    }
    let doc = Document::parse(
        "note",
        Path::new("note.hmd"),
        Format::Hmd,
        "[[bird]] {label: \"foo,bar}\", nested: {tags: [owl, robin]}} and [[tree]]",
    )
    .unwrap();
    assert_eq!(doc.references.len(), 2);
    assert_eq!(
        doc.references[0].data.path("label").and_then(Data::as_str),
        Some("foo,bar}")
    );
    assert!(
        matches!(doc.references[0].data.path("nested.tags"), Some(Data::List(items)) if items.len() == 2)
    );
    for source in [
        "---\ntitle: unclosed",
        "[[bird]] {label: [broken}",
        "[[bird]] {label: unclosed",
    ] {
        assert!(Document::parse("note", Path::new("note.hmd"), Format::Hmd, source).is_err());
    }
}

struct Memory;

impl VaultSource for Memory {
    fn read(&self) -> std::io::Result<Snapshot> {
        Ok(Snapshot {
            root: "memory".into(),
            documents: [
                ("b/note.hmd", "# Second"),
                ("a/note.hmd", "# First\n[[b/note]] {relation: related}"),
                ("invalid.hmd", "---\ntags: [broken\n---\n# Invalid"),
                ("relation.hmd", "---\nmetadata:\n  knowledge:\n    type: Relation\n    relation: {source: a/note, target: b/note}\n---\n# Relation"),
            ].into_iter().map(|(path, text)| SourceDocument { path: path.into(), format: Format::Hmd, text: Ok(text.into()) }).collect(),
        })
    }
}

#[test]
fn memory_sources_get_the_same_names_knowledge_and_extension_metadata() {
    let vault = hql::vault::load_source(&Memory).unwrap();
    assert_eq!(
        vault
            .cards
            .iter()
            .map(|card| card.name())
            .collect::<Vec<_>>(),
        ["a/note", "b/note", "relation"]
    );
    assert_eq!(
        vault.resolve("relation").unwrap().kind,
        hql::document::Kind::Relation
    );
    assert_eq!(vault.edges.len(), 2);
    assert_eq!(
        vault
            .resolve("a/note")
            .unwrap()
            .metadata
            .path("lexical.indexed"),
        Some(&Data::Bool(true))
    );
    assert_eq!(vault.warnings.len(), 1);
    assert!(vault.warnings[0].contains("memory/invalid.hmd"));
    assert!(vault.warnings[0].contains("invalid YAML"));
    assert!(vault.resolve("invalid").is_none());
}
