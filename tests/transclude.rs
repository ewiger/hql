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
    assert!(rendered.document.contains(&format!(
        "```\n{OUTPUT_OPEN}\n- Bearer tokens\n- Session cookies\n{OUTPUT_CLOSE}\n"
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
    assert_eq!(
        rendered.document,
        format!("```hql#eval after\n1 + 1\n```\n{OUTPUT_OPEN}\n2\n{OUTPUT_CLOSE}\n\n4\n")
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
        rendered
            .document
            .contains(&format!("{OUTPUT_OPEN}\nerror:")),
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
