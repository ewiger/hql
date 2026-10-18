//! HQL written inside a document, and the document rendering builds from it.
//!
//! This is the second host after the console: a card carries a query, and
//! rendering the card runs it. A to-do list becomes a query rather than a
//! list somebody maintains.

use hql::reporting::Mode;
use hql::transclude::{self, Emit, OUTPUT_CLOSE, OUTPUT_OPEN};
use hql::vault;
use std::path::Path;

fn fixture() -> vault::Vault {
    vault::load(Path::new("tests/fixtures/vault")).expect("the fixture vault loads")
}

fn board() -> String {
    std::fs::read_to_string("tests/fixtures/board.hmd").expect("the board document")
}

#[test]
fn replacing_compiles_the_cells_away_into_markdown() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict, Emit::Replace);
    assert_eq!(rendered.blocks, 2);
    assert!(rendered.complete);
    assert!(!rendered.reports.has_errors(), "{:?}", rendered.reports);

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
    let source = "```hql#eval after\n1 + 1\n```\n\n```hql\n2 + 2\n```\n";
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
    let source = "# Notes\n\n```hql\ncards | nosuchstep\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::Replace);
    assert_eq!(rendered.blocks, 1);
    assert!(rendered.reports.has_errors());
    assert!(!rendered.complete);
    // The cell is left as written rather than lost.
    assert_eq!(rendered.document, source);
}

#[test]
fn a_failed_cell_under_after_shows_its_error_as_output() {
    let source = "```hql\ncards | nosuchstep\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict, Emit::After);
    assert!(rendered.reports.has_errors());
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
    let vault = vault::load(Path::new("examples/birds/wiki")).expect("the example wiki");
    assert!(vault.warnings.is_empty(), "{:?}", vault.warnings);
    let mut blocks = 0;
    for card in &vault.cards {
        for reference in &card.document.references {
            assert!(
                vault.resolve(&reference.target).is_some(),
                "{}: unresolved {}",
                card.name(),
                reference.target
            );
        }
        let source = std::fs::read_to_string(&card.document.path).unwrap();
        let rendered = transclude::render(&source, &vault, Mode::Strict, Emit::After);
        assert!(
            rendered.reports.is_empty(),
            "{}: {:?}",
            card.name(),
            rendered.reports
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
            assert!(!built.document.contains("```hql"), "{}", card.name());
        }
    }
    assert!(blocks >= 3, "the collection notebook's queries must run");
}

fn after(source: &str) -> transclude::Rendered {
    transclude::render(source, &fixture(), Mode::Strict, Emit::After)
}

fn replaced(source: &str) -> transclude::Rendered {
    transclude::render(source, &fixture(), Mode::Strict, Emit::Replace)
}

#[test]
fn a_cell_reads_another_by_its_id_whatever_order_they_are_written_in() {
    let source = "```hql\ntotal + 1\n```\n\n```hql id=total\n[1, 2, 3] | sum\n```\n";
    let rendered = replaced(source);
    assert!(rendered.complete, "{:?}", rendered.reports);
    assert_eq!(rendered.document, "7\n\n6\n");
    assert_eq!(rendered.cells[0].ty.as_deref(), Some("Int"));
}

#[test]
fn a_cell_exports_its_value_and_keeps_its_bindings() {
    let source = "```hql id=open\nhelper = 2\n[1, helper]\n```\n\n```hql\nhelper\n```\n";
    let rendered = replaced(source);
    assert!(!rendered.complete);
    let message = format!("{:?}", rendered.reports);
    assert!(message.contains("`helper` is not bound"), "{message}");
    let shadowed = "```hql id=n\n1\n```\n\n```hql\nn = 5\nn + 1\n```\n";
    assert_eq!(replaced(shadowed).document, "1\n\n6\n");
}

#[test]
fn a_cycle_and_a_duplicate_id_are_check_errors_naming_the_cells() {
    let cycle = replaced("```hql id=a\nb\n```\n\n```hql id=b\na\n```\n");
    let message = format!("{:?}", cycle.reports);
    assert!(
        message.contains("`a`, `b` read each other in a cycle"),
        "{message}"
    );
    assert_eq!(cycle.blocks, 0);
    let twice = replaced("```hql id=a\n1\n```\n\n```hql id=a\n2\n```\n");
    let message = format!("{:?}", twice.reports);
    assert!(
        message.contains("cells 0 and 1 both have the id `a`"),
        "{message}"
    );
    assert!(!twice.complete);
}

#[test]
fn a_type_error_anywhere_stops_the_document_before_any_cell_runs() {
    let source = "```hql id=a\n[1] | debug\n```\n\n```hql\na | nosuchstep\n```\n";
    let rendered = after(source);
    assert_eq!(rendered.cells[0].status, transclude::Status::NotRun);
    assert_eq!(rendered.cells[1].status, transclude::Status::Error);
    assert!(
        rendered
            .reports
            .iter()
            .all(|report| report.code.as_deref() != Some("debug"))
    );
}

#[test]
fn a_cell_waiting_on_a_failed_one_is_skipped_and_the_rest_still_run() {
    let source = "```hql id=a\n1 / 0\n```\n\n```hql id=b\na + 1\n```\n\n```hql\n2 + 2\n```\n";
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
    let source = "```hql id=base hidden\n10\n```\n\n```hql\nbase * 2\n```\n";
    assert_eq!(replaced(source).document, "20\n");
    let notebook = after(source);
    assert_eq!(notebook.document.matches(OUTPUT_OPEN).count(), 1);
}

#[test]
fn an_output_records_its_cell_and_the_hash_of_the_code_behind_it() {
    let source = "```hql id=base\n10\n```\n\n```hql\nbase * 2\n```\n";
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
    let source = "Intro\n\n<!-- hql:output id=gone hash=000000000000 status=ok -->\nold\n<!-- /hql:output -->\n\n```hql\n1\n```\n";
    let rendered = after(source);
    assert!(!rendered.document.contains("old"), "{}", rendered.document);
    let message = format!("{:?}", rendered.reports);
    assert!(message.contains("no cell directly above it"), "{message}");
}

#[test]
fn an_unknown_attribute_is_a_warning_and_ignored() {
    let rendered = replaced("```hql id=x shiny\n1\n```\n");
    assert_eq!(rendered.document, "1\n");
    let message = format!("{:?}", rendered.reports);
    assert!(
        message.contains("`shiny` is not a cell attribute"),
        "{message}"
    );
}

#[test]
fn a_cells_report_points_into_the_document() {
    let source = "# Title\n\n```hql\ncards | nosuchstep\n```\n";
    let rendered = replaced(source);
    let report = rendered
        .reports
        .iter()
        .find(|report| report.message.contains("nosuchstep"))
        .unwrap();
    assert_eq!(&source[report.span.clone()], "nosuchstep");
}

#[test]
fn one_cell_runs_with_what_it_reads_and_the_rest_is_left_as_written() {
    let source = "```hql id=a\n1\n```\n<!-- hql:output id=a hash=000000000000 status=ok -->\nstale\n<!-- /hql:output -->\n\n```hql id=b\na + 1\n```\n\n```hql id=c\n99\n```\n<!-- hql:output id=c hash=000000000000 status=ok -->\nkept\n<!-- /hql:output -->\n";
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
