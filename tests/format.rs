//! `format` writes a value as text of a presentation format, and a vault's
//! `hql.toml` chooses which renderer writes a type in a format.

use hql::reporting::Mode;
use hql::types::Value;
use hql::vault::Vault;
use std::path::Path;

fn fixture() -> Vault {
    hql::vault::load(Path::new("tests/fixtures/vault")).unwrap()
}

/// The text a query writes and the name of its format type.
fn written(source: &str, vault: &Vault) -> (String, String) {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(!outcome.failed(), "{source}: {:?}", outcome.diagnostics);
    match outcome.value {
        Some(Value::Formatted(formatted)) => {
            (formatted.text.clone(), formatted.format.0.to_owned())
        }
        other => panic!("{source}: expected formatted text, got {other:?}"),
    }
}

fn refused(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(outcome.failed(), "{source}");
    format!("{:?}", outcome.diagnostics)
}

#[test]
fn a_shorthand_is_format_with_its_format_fixed() {
    let vault = fixture();
    for (shorthand, format) in [
        ("text", "Text"),
        ("markdown", "Markdown"),
        ("hmd", "Hmd"),
        ("json", "Json"),
        ("yaml", "Yaml"),
        ("csv", "Csv"),
    ] {
        let short = written(&format!("[1, 2, 3] | {shorthand}"), &vault);
        let long = written(&format!("[1, 2, 3] | format({format})"), &vault);
        assert_eq!(short, long, "{shorthand}");
        assert_eq!(short.1, format);
    }
}

#[test]
fn formatted_text_is_a_string_a_later_step_reads() {
    let vault = fixture();
    let outcome = hql::run(
        "([1, 2] | json) == \"[\\n  1,\\n  2\\n]\"",
        &vault,
        Mode::Strict,
    );
    assert_eq!(
        outcome.value,
        Some(Value::Bool(true)),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn limit_keeps_the_first_elements_and_says_how_many_were_cut() {
    let vault = fixture();
    let (text, _) = written("[1, 2, 3, 4] | format(Text, limit = 2)", &vault);
    assert!(text.ends_with("… 2 more not shown"), "{text}");
    assert!(!text.contains('3'), "{text}");
    let (text, _) = written("[1, 2, 3] | format(Json, limit = 1)", &vault);
    assert_eq!(text, "[\n  1\n]");
}

#[test]
fn a_graph_has_a_default_renderer_in_every_graph_format() {
    let vault = fixture();
    let (text, _) = written("cards | Graph | mermaid", &vault);
    assert!(text.starts_with("graph LR"), "{text}");
    let (text, _) = written("cards | Graph | d2", &vault);
    assert!(text.contains(" -> "), "{text}");
    let (text, _) = written("cards | Graph | format(Markdown)", &vault);
    assert!(text.starts_with("```d2\n"), "{text}");
    let (text, _) = written("cards | Graph | json", &vault);
    assert!(text.contains("\"nodes\""), "{text}");
}

#[test]
fn a_renderer_argument_overrides_the_default() {
    let vault = fixture();
    let (text, _) = written(
        "cards | Graph | format(Markdown, renderer = \"mermaid\")",
        &vault,
    );
    assert!(text.starts_with("```mermaid\n"), "{text}");
    assert!(
        refused("[1] | format(Text, renderer = \"mermaid\")", &vault).contains("does not write")
    );
}

#[test]
fn a_type_no_renderer_writes_in_a_format_is_refused() {
    let vault = fixture();
    assert!(refused("1 | format(D2)", &vault).contains("no renderer writes Int as D2"));
    assert!(refused("1 | format(Int)", &vault).contains("Format"));
    // Refused before the program runs, so nothing is evaluated.
    let refusal = hql::check_in("1 | mermaid", &vault).expect_err("checked");
    assert!(
        refusal
            .message()
            .contains("no renderer writes Int as Mermaid")
    );
}

#[test]
fn text_of_one_format_is_itself_plain_text_or_a_fence() {
    let vault = fixture();
    let (d2, _) = written("cards | Graph | d2", &vault);
    let (fenced, format) = written("cards | Graph | d2 | markdown", &vault);
    assert_eq!(fenced, format!("```d2\n{d2}\n```"));
    assert_eq!(format, "Markdown");
    let (markdown, _) = written("[1, 2] | markdown", &vault);
    assert_eq!(
        written("[1, 2] | markdown | hmd", &vault),
        (markdown.clone(), "Hmd".to_owned())
    );
    assert_eq!(written("[1, 2] | hmd | markdown", &vault).0, markdown);
    let (json, _) = written("[1, 2] | json", &vault);
    assert_eq!(written("[1, 2] | json | text", &vault).0, json);
    // Encoded text is a String, whatever it was encoded in.
    assert!(refused("[1, 2] | json | d2", &vault).contains("no renderer writes String as D2"));
    // Encoded again in another format, the text is one String value.
    let (again, format) = written("[1, 2] | markdown | json", &vault);
    assert_eq!(again, "\"- 1\\n- 2\"");
    assert_eq!(format, "Json");
}

fn configured(toml: &str) -> std::io::Result<Vault> {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("a.hmd"), "# A\n\nSee [[b]].\n").unwrap();
    std::fs::write(directory.path().join("b.hmd"), "# B\n").unwrap();
    std::fs::write(directory.path().join("hql.toml"), toml).unwrap();
    hql::vault::load(directory.path())
}

#[test]
fn a_vault_chooses_the_renderer_for_a_type_in_a_format() {
    let vault = configured("[format.Graph]\nmarkdown = \"mermaid\"\n").unwrap();
    let (text, _) = written("cards | Graph | markdown", &vault);
    assert!(text.starts_with("```mermaid\n"), "{text}");
    // Other formats keep their defaults.
    let (text, _) = written("cards | Graph | d2", &vault);
    assert!(!text.contains("graph LR"), "{text}");
}

#[test]
fn a_configuration_naming_what_does_not_exist_is_refused_on_load() {
    for (toml, message) in [
        (
            "[format.Graph]\nmarkdown = \"cytoscape\"\n",
            "unknown renderer `cytoscape`",
        ),
        ("[format.Graph]\npdf = \"d2\"\n", "unknown format `pdf`"),
        ("[format.Graph]\njson = \"d2\"\n", "does not write Json"),
        ("[format]\nGraph = \"d2\"\n", "maps formats to renderers"),
        (
            "[format.Int]\nmarkdown = \"d2\"\n",
            "writes a Graph, not Int",
        ),
    ] {
        let error = configured(toml).expect_err(toml);
        assert!(error.to_string().contains(message), "{toml}: {error}");
    }
}

#[test]
fn graph_is_a_constructor_and_a_pipeline_step_of_that_name() {
    let vault = fixture();
    let (piped, _) = written("cards | Graph | json", &vault);
    let (called, _) = written("Graph(cards) | json", &vault);
    assert_eq!(piped, called);
    for name in ["bearer-tokens", "oauth", "sessions"] {
        assert!(piped.contains(&format!("\"id\": \"{name}\"")), "{piped}");
    }
    let (text, _) = written("cards | Graph | format(Markdown, limit = 2)", &vault);
    assert!(text.ends_with("more not shown"), "{text}");
    assert!(refused("[1, 2] | Graph", &vault).contains("projected from cards"));
}

#[test]
fn expand_traverses_at_least_one_step() {
    let vault = fixture();
    assert!(refused("cards | expand(depth = 0)", &vault).contains("`Graph(xs)`"));
    assert!(refused("cards | expand(0)", &vault).contains("`Graph(xs)`"));
}

#[test]
fn a_graph_says_why_each_node_is_present() {
    let vault = fixture();
    let (text, _) = written(
        "import lexical\ncards | lexical(\"bearer token authorization\") | take(2) | expand(depth = 1) | text",
        &vault,
    );
    assert!(text.contains("(retrieved "), "{text}");
    assert!(text.contains("(expanded from "), "{text}");
}

#[test]
fn limit_applies_to_a_sequence_the_vault_produced() {
    let vault = fixture();
    let (text, _) = written("cards | map(c => c.name) | format(Text, limit = 2)", &vault);
    assert!(text.ends_with("more not shown"), "{text}");
}

/// Two rows of a declared record, the table's columns named against the
/// order the record declares them in.
const BOOKS: &str = "type Book { title: String, pages: Int, year: Int }\n\
    Table([Book(title = \"Dune\", pages = 412, year = 1965), \
    Book(title = \"Hobbit, The\", pages = 310, year = 1937)], columns = [\"year\", \"title\"])";

#[test]
fn a_table_writes_its_columns_in_the_order_named_in_every_format() {
    let vault = fixture();
    let (text, format) = written(&format!("{BOOKS} | markdown"), &vault);
    assert_eq!(format, "Markdown");
    assert_eq!(
        text,
        "| year | title       |\n| ---- | ----------- |\n| 1965 | Dune        |\n| 1937 | Hobbit, The |"
    );
    let (text, _) = written(&format!("{BOOKS} | csv"), &vault);
    assert_eq!(text, "year,title\r\n1965,Dune\r\n1937,\"Hobbit, The\"");
    let (text, _) = written(&format!("{BOOKS} | json"), &vault);
    assert_eq!(
        text,
        "[\n  {\n    \"year\": 1965,\n    \"title\": \"Dune\"\n  },\n  {\n    \"year\": 1937,\n    \"title\": \"Hobbit, The\"\n  }\n]"
    );
    let (text, _) = written(&format!("{BOOKS} | text"), &vault);
    assert_eq!(
        text,
        "year  title\n----  -----------\n1965  Dune\n1937  Hobbit, The"
    );
}

#[test]
fn a_table_without_columns_takes_the_first_rows_fields_in_declared_order() {
    let vault = fixture();
    let (text, _) = written(
        "type Book { title: String, pages: Int }\n\
         [Book(title = \"Dune\", pages = 412)] | Table | csv",
        &vault,
    );
    assert_eq!(text, "title,pages\r\nDune,412");
    // Over Data, a field a row does not have is an empty cell.
    let (text, _) = written(
        "Table([{a: 1, b: 2}, {a: 3}], columns = [\"b\", \"a\"]) | csv",
        &vault,
    );
    assert_eq!(text, "b,a\r\n2,1\r\n,3");
}

#[test]
fn a_table_column_naming_no_field_of_the_record_is_refused() {
    let vault = fixture();
    let refusal = hql::check_in(
        "type Book { title: String }\nTable([Book(title = \"Dune\")], columns = [\"title\", \"isbn\"])",
        &vault,
    )
    .expect_err("checked");
    assert!(
        refusal
            .message()
            .contains("the column `isbn` names no field of Book"),
        "{}",
        refusal.message()
    );
    assert!(refused("Table([1, 2])", &vault).contains("a Table row is a record"));
}

#[test]
fn a_table_renderer_writes_only_a_table() {
    let vault = fixture();
    let (text, _) = written(
        "Table([{a: 1}]) | format(Markdown, renderer = \"pipe\")",
        &vault,
    );
    assert_eq!(text, "| a   |\n| --- |\n| 1   |");
    for (renderer, format) in [
        ("pipe", "Markdown"),
        ("aligned", "Text"),
        ("rows", "Csv"),
        ("records", "Json"),
    ] {
        let source = format!("[{{a: 1}}] | format({format}, renderer = \"{renderer}\")");
        let refusal = hql::check_in(&source, &vault).expect_err(&source);
        assert!(
            refusal
                .message()
                .contains(&format!("the renderer `{renderer}` writes a Table")),
            "{}",
            refusal.message()
        );
    }
    let error = configured("[format.Data]\nmarkdown = \"pipe\"\n").expect_err("refused");
    assert!(
        error.to_string().contains("writes a Table, not Data"),
        "{error}"
    );
    let vault = configured("[format.Table]\njson = \"records\"\n").unwrap();
    let (text, _) = written("Table([{a: 1}]) | json", &vault);
    assert_eq!(text, "[\n  {\n    \"a\": 1\n  }\n]");
}

#[test]
fn table_as_a_step_is_refused_naming_the_constructor() {
    let vault = fixture();
    let refusal = refused("[{a: 1}] | table", &vault);
    assert!(refusal.contains("Table(xs, columns = [...])"), "{refusal}");
    assert_eq!(hql::builtins::nearest("table"), Some("Table"));
}
