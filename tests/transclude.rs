//! HQL written inside a document, and the document rendering builds from it.
//!
//! This is the second host after the console: a card carries a query, and
//! rendering the card runs it. A to-do list becomes a query rather than a
//! list somebody maintains.

use hql::reporting::Mode;
use hql::transclude::{self, Emit, OUTPUT_CLOSE, OUTPUT_OPEN};
use std::path::Path;

fn fixture() -> hql::environment::RunEnvironment {
    hql::environment::load(Path::new("tests/fixtures/vault")).expect("the fixture vault loads")
}

fn board() -> String {
    std::fs::read_to_string("tests/fixtures/board.hmd").expect("the board document")
}

#[test]
fn replacing_compiles_the_cells_away_into_markdown() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict, Emit::Replace);
    assert_eq!(rendered.blocks, 2);
    assert!(rendered.complete);
    assert!(
        !hql::diagnostics::reaches(&rendered.diagnostics, hql::diagnostics::Severity::Error),
        "{:?}",
        rendered.diagnostics
    );

    // No cell is left: the document reads as if written by hand.
    assert!(!rendered.document.contains("```"), "{}", rendered.document);
    assert!(!rendered.document.contains(OUTPUT_OPEN));
    // A sequence without a presentation is a Markdown list.
    assert!(
        rendered
            .document
            .contains("- Bearer tokens\n- Session cookies")
    );
    // Everything that is done stays out of the list. (OAuth appears further
    // down, in the ranking, which is a different cell.)
    let todo = rendered
        .document
        .split("## Closest")
        .next()
        .expect("the to-do section");
    assert!(!todo.contains("OAuth 2.0"), "{todo}");
}

#[test]
fn a_table_in_a_document_is_a_pipe_table() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict, Emit::Replace);
    assert!(
        rendered
            .document
            .contains("| rank | score  | card          | title         |\n| ---- |"),
        "{}",
        rendered.document
    );
    assert!(rendered.document.contains("| bearer-tokens |"));
}

#[test]
fn after_keeps_the_cell_and_marks_its_output() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict, Emit::After);
    assert!(rendered.document.contains("```hql#eval"));
    assert!(
        rendered
            .document
            .contains(&format!("```\n{OUTPUT_OPEN} hash=")),
        "{}",
        rendered.document
    );
    assert!(rendered.document.contains(&format!(
        "status=ok -->\n- Bearer tokens\n- Session cookies\n{OUTPUT_CLOSE}\n"
    )));
    assert_eq!(rendered.document.matches(OUTPUT_OPEN).count(), 2);
}

#[test]
fn rendering_after_twice_leaves_what_rendering_once_did() {
    let vault = fixture();
    let once = transclude::render(&board(), &vault, Mode::Strict, Emit::After);
    let twice = transclude::render(&once.document, &vault, Mode::Strict, Emit::After);
    assert_eq!(once.document, twice.document);
    assert_eq!(twice.blocks, 2);
}

#[test]
fn replacing_a_rendered_notebook_drops_its_old_output() {
    let vault = fixture();
    let notebook = transclude::render(&board(), &vault, Mode::Strict, Emit::After);
    let built = transclude::render(&notebook.document, &vault, Mode::Strict, Emit::Replace);
    let direct = transclude::render(&board(), &vault, Mode::Strict, Emit::Replace);
    assert_eq!(built.document, direct.document);
}

#[test]
fn a_cell_may_name_its_own_mode() {
    let source = "```hql#eval after\n1 + 1\n```\n\n```hql#eval\n2 + 2\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::Replace);
    let hash = &rendered.cells[0].hash;
    assert_eq!(
        rendered.document,
        format!(
            "```hql#eval after\n1 + 1\n```\n{OUTPUT_OPEN} hash={hash} status=ok -->\n2\n{OUTPUT_CLOSE}\n\n4\n"
        )
    );
}

#[test]
fn a_failed_cell_under_replace_leaves_the_build_incomplete() {
    let source = "# Notes\n\n```hql#eval\ncards | nosuchstep\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::Replace);
    assert_eq!(rendered.blocks, 1);
    assert!(hql::diagnostics::reaches(
        &rendered.diagnostics,
        hql::diagnostics::Severity::Error
    ));
    assert!(!rendered.complete);
    // The cell is left as written rather than lost.
    assert_eq!(rendered.document, source);
}

#[test]
fn a_failed_cell_under_after_shows_its_error_as_output() {
    let source = "```hql#eval\ncards | nosuchstep\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::After);
    assert!(hql::diagnostics::reaches(
        &rendered.diagnostics,
        hql::diagnostics::Severity::Error
    ));
    assert!(rendered.complete);
    assert!(
        rendered.document.contains("status=error -->\nerror:"),
        "{}",
        rendered.document
    );
}

#[test]
fn a_fence_that_is_not_hql_is_left_alone() {
    let source = "```rust\nfn main() {}\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::Replace);
    assert_eq!(rendered.blocks, 0);
    assert_eq!(rendered.document, source);
}

#[test]
fn the_birds_example_wiki_resolves_and_renders_every_query() {
    let vault = hql::environment::load(Path::new("examples/birds/wiki")).expect("the example wiki");
    assert!(vault.warnings().is_empty(), "{:?}", vault.warnings());
    let documents = vault.vault.as_ref().expect("the example wiki is a vault");
    let mut blocks = 0;
    for card in vault.cards() {
        for reference in &card.document.references {
            assert!(
                vault.resolve(&reference.target).is_some(),
                "{}: unresolved {}",
                card.name(),
                reference.target
            );
        }
        let source = std::fs::read_to_string(documents.file_of(&card.document)).unwrap();
        let rendered = transclude::render(&source, &vault, Mode::Strict, Emit::After);
        assert!(
            rendered.diagnostics.is_empty(),
            "{}: {:?}",
            card.name(),
            rendered.diagnostics
        );
        blocks += rendered.blocks;
        assert_eq!(
            transclude::render(&rendered.document, &vault, Mode::Strict, Emit::After).document,
            rendered.document,
            "{}: rendering is idempotent",
            card.name()
        );
        let built = transclude::render(&source, &vault, Mode::Strict, Emit::Replace);
        assert!(built.complete, "{}", card.name());
        if built.blocks > 0 {
            assert!(!built.document.contains("```hql#"), "{}", card.name());
        }
    }
    assert!(blocks >= 3, "the collection notebook's queries must run");
}

#[test]
fn the_todo_example_board_renders_its_lists() {
    let vault = hql::environment::load(Path::new("examples/todo/wiki")).expect("the example wiki");
    assert!(vault.warnings().is_empty(), "{:?}", vault.warnings());
    let source = std::fs::read_to_string("examples/todo/wiki/todo.hmd").unwrap();
    let built = transclude::render(&source, &vault, Mode::Strict, Emit::Replace);
    assert!(built.complete, "{:?}", built.diagnostics);
    assert_eq!(built.blocks, 2);
    assert!(
        built.document.contains(
            "## Still to do\n\n\
             - Fix broken backlinks\n\
             - Write HQL documentation\n\
             - Publish HyperMarkdown 0.4.2 announcement\n"
        ),
        "{}",
        built.document
    );
    assert!(
        built.document.contains(
            "## Architecture decisions\n\n\
             - {date: \"2026-10-10\", status: \"accepted\", title: \"Event execution model\"}\n\
             - {date: \"2026-10-18\", status: \"accepted\", title: \"Reduce and merge\"}\n\
             - {date: \"2026-10-19\", status: \"proposed\", title: \"Stream IO model\"}\n"
        ),
        "{}",
        built.document
    );
}

fn after(source: &str) -> transclude::Rendered {
    transclude::render(source, &fixture(), Mode::Strict, Emit::After)
}

fn replaced(source: &str) -> transclude::Rendered {
    transclude::render(source, &fixture(), Mode::Strict, Emit::Replace)
}

#[test]
fn a_cell_reads_another_by_its_id_whatever_order_they_are_written_in() {
    let source = "```hql#eval\ntotal + 1\n```\n\n```hql#eval id=total\n[1, 2, 3] | sum\n```\n";
    let rendered = replaced(source);
    assert!(rendered.complete, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.document, "7\n\n6\n");
    assert_eq!(rendered.cells[0].ty.as_deref(), Some("Int"));
}

#[test]
fn a_cell_exports_its_value_and_keeps_its_bindings() {
    let source = "```hql#eval id=open\nhelper = 2\n[1, helper]\n```\n\n```hql#eval\nhelper\n```\n";
    let rendered = replaced(source);
    assert!(!rendered.complete);
    let message = format!("{:?}", rendered.diagnostics);
    assert!(message.contains("`helper` is not bound"), "{message}");
    let shadowed = "```hql#eval id=n\n1\n```\n\n```hql#eval\nn = 5\nn + 1\n```\n";
    assert_eq!(replaced(shadowed).document, "1\n\n6\n");
}

#[test]
fn a_cycle_and_a_duplicate_id_are_check_errors_naming_the_cells() {
    let cycle = replaced("```hql#eval id=a\nb\n```\n\n```hql#eval id=b\na\n```\n");
    let message = format!("{:?}", cycle.diagnostics);
    assert!(
        message.contains("`a`, `b` read each other in a cycle"),
        "{message}"
    );
    assert_eq!(cycle.blocks, 0);
    let twice = replaced("```hql#eval id=a\n1\n```\n\n```hql#eval id=a\n2\n```\n");
    let message = format!("{:?}", twice.diagnostics);
    assert!(
        message.contains("cells 0 and 1 both have the id `a`"),
        "{message}"
    );
    assert!(!twice.complete);
}

#[test]
fn a_type_error_anywhere_stops_the_document_before_any_cell_runs() {
    let source = "```hql#eval id=a\n[1] | size\n```\n\n```hql#eval\na | nosuchstep\n```\n";
    let rendered = after(source);
    assert_eq!(rendered.cells[0].status, transclude::Status::NotRun);
    assert_eq!(rendered.cells[1].status, transclude::Status::Error);
}

#[test]
fn a_cell_waiting_on_a_failed_one_is_skipped_and_the_rest_still_run() {
    let source =
        "```hql#eval id=a\n1 / 0\n```\n\n```hql#eval id=b\na + 1\n```\n\n```hql#eval\n2 + 2\n```\n";
    let rendered = after(source);
    let statuses: Vec<_> = rendered.cells.iter().map(|cell| cell.status).collect();
    use transclude::Status::{Error, Ok, Skipped};
    assert_eq!(statuses, [Error, Skipped, Ok]);
    assert!(
        rendered
            .document
            .contains("status=skipped -->\nskipped: a failed\n"),
        "{}",
        rendered.document
    );
    assert!(rendered.document.contains("status=ok -->\n4\n"));
}

#[test]
fn a_hidden_cell_runs_and_writes_nothing() {
    let source = "```hql#eval id=base hidden\n10\n```\n\n```hql#eval\nbase * 2\n```\n";
    assert_eq!(replaced(source).document, "20\n");
    let notebook = after(source);
    assert_eq!(notebook.document.matches(OUTPUT_OPEN).count(), 1);
}

#[test]
fn a_query_cell_is_a_hidden_cell() {
    let source = "```hql#query id=base\n10\n```\n\n```hql#eval\nbase * 2\n```\n";
    assert_eq!(replaced(source).document, "20\n");
    let notebook = after(source);
    assert_eq!(notebook.cells.len(), 2);
    assert_eq!(notebook.document.matches(OUTPUT_OPEN).count(), 1);
    assert!(
        notebook
            .document
            .contains("```hql#query id=base\n10\n```\n\n```hql#eval")
    );
    // Writing `hidden` on a query cell says it twice and changes nothing.
    let twice = source.replace("id=base", "id=base hidden");
    assert_eq!(after(&twice).document.matches(OUTPUT_OPEN).count(), 1);
    assert!(after(&twice).diagnostics.is_empty());
}

#[test]
fn a_plain_hql_fence_is_a_listing() {
    let source = "```hql#eval\n1 + 1\n```\n\n```hql\ncards | nosuchstep\n```\n";
    let built = replaced(source);
    assert!(built.complete, "{:?}", built.diagnostics);
    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    assert_eq!(built.blocks, 1);
    assert_eq!(built.document, "2\n\n```hql\ncards | nosuchstep\n```\n");
    let notebook = after(source);
    assert_eq!(notebook.cells.len(), 1);
    assert!(
        notebook
            .document
            .ends_with("```hql\ncards | nosuchstep\n```\n")
    );
    assert_eq!(notebook.document.matches(OUTPUT_OPEN).count(), 1);
    assert_eq!(after(&notebook.document).document, notebook.document);
}

#[test]
fn an_id_on_a_listing_names_nothing() {
    let source = "```hql id=base\n10\n```\n\n```hql#eval\nbase * 2\n```\n";
    let rendered = after(source);
    assert_eq!(rendered.cells.len(), 1);
    assert!(rendered.document.starts_with("```hql id=base\n10\n```\n"));
    assert!(
        rendered.document.contains("status=error"),
        "{}",
        rendered.document
    );
    let message = format!("{:?}", rendered.diagnostics);
    assert!(message.contains("base"), "{message}");
}

#[test]
fn the_hypermarkdown_notebook_leaves_its_listing_unevaluated() {
    let vault = hql::environment::load(Path::new("tests/fixtures/hmd-notebook"))
        .expect("the notebook vault loads");
    let source = std::fs::read_to_string("tests/fixtures/hmd-notebook/contents.hmd").unwrap();
    let rendered = transclude::render(&source, &vault, Mode::Strict, Emit::After);
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    // Its saved outputs are current, so rendering writes the card unchanged:
    // the listing gets no output and the query cell none either.
    assert_eq!(rendered.document, source);
    assert_eq!(rendered.cells.len(), 4);
    assert!(rendered.document.ends_with("```hql\ncards | size\n```\n"));
    let built = transclude::render(&source, &vault, Mode::Strict, Emit::Replace);
    assert!(built.complete, "{:?}", built.diagnostics);
    assert!(!built.document.contains("```hql#"), "{}", built.document);
    assert!(built.document.ends_with("```hql\ncards | size\n```\n"));
}

#[test]
fn an_output_records_its_cell_and_the_hash_of_the_code_behind_it() {
    let source = "```hql#eval id=base\n10\n```\n\n```hql#eval\nbase * 2\n```\n";
    let rendered = after(source);
    let base = &rendered.cells[0].hash;
    assert_eq!(base.len(), 12);
    assert!(
        rendered
            .document
            .contains(&format!("{OUTPUT_OPEN} id=base hash={base} status=ok -->"))
    );
    // Editing a cell changes its hash and the hash of every cell reading it.
    let edited = after(&source.replace("10", "11"));
    assert_ne!(edited.cells[0].hash, rendered.cells[0].hash);
    assert_ne!(edited.cells[1].hash, rendered.cells[1].hash);
    // Trailing whitespace is not code.
    let spaced = after(&source.replace("10\n", "10   \n"));
    assert_eq!(spaced.cells[0].hash, rendered.cells[0].hash);
}

#[test]
fn an_output_nobody_owns_is_removed_with_a_warning() {
    let source = "Intro\n\n<!-- hql:output id=gone hash=000000000000 status=ok -->\nold\n<!-- /hql:output -->\n\n```hql#eval\n1\n```\n";
    let rendered = after(source);
    assert!(!rendered.document.contains("old"), "{}", rendered.document);
    let message = format!("{:?}", rendered.diagnostics);
    assert!(message.contains("no cell directly above it"), "{message}");
}

#[test]
fn an_unknown_attribute_is_a_warning_and_ignored() {
    let rendered = replaced("```hql#eval id=x shiny\n1\n```\n");
    assert_eq!(rendered.document, "1\n");
    let message = format!("{:?}", rendered.diagnostics);
    assert!(
        message.contains("`shiny` is not a cell attribute"),
        "{message}"
    );
}

#[test]
fn a_cells_report_points_into_the_document() {
    let source = "# Title\n\n```hql#eval\ncards | nosuchstep\n```\n";
    let rendered = replaced(source);
    let report = rendered
        .diagnostics
        .iter()
        .find(|report| report.message.contains("nosuchstep"))
        .unwrap();
    assert_eq!(&source[report.span.clone()], "nosuchstep");
}

#[test]
fn one_cell_runs_with_what_it_reads_and_the_rest_is_left_as_written() {
    let source = "```hql#eval id=a\n1\n```\n<!-- hql:output id=a hash=000000000000 status=ok -->\nstale\n<!-- /hql:output -->\n\n```hql#eval id=b\na + 1\n```\n\n```hql#eval id=c\n99\n```\n<!-- hql:output id=c hash=000000000000 status=ok -->\nkept\n<!-- /hql:output -->\n";
    let rendered =
        transclude::render_cell(source, &fixture(), Mode::Strict, Emit::After, Some("b")).unwrap();
    assert!(
        rendered.document.contains("status=ok -->\n2\n"),
        "{}",
        rendered.document
    );
    assert!(rendered.document.contains("kept"));
    assert_eq!(rendered.cells[2].status, transclude::Status::NotRun);
    assert!(
        transclude::render_cell(source, &fixture(), Mode::Strict, Emit::After, Some("zz")).is_err()
    );
}
