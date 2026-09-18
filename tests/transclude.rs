//! HQL written inside a document, and the answer transcluded beneath it.
//!
//! This is the second host after the console: a card carries a query, and
//! rendering the card runs it. A to-do list becomes a query rather than a
//! list somebody maintains.

use hql::reporting::Mode;
use hql::{transclude, vault};
use std::path::Path;

fn fixture() -> vault::Vault {
    vault::load(Path::new("tests/fixtures/vault")).expect("the fixture vault loads")
}

fn board() -> String {
    std::fs::read_to_string("tests/fixtures/board.hmd").expect("the board document")
}

#[test]
fn a_to_do_list_is_a_query_and_its_answer_is_transcluded() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict);
    assert_eq!(rendered.blocks, 2);
    assert!(!rendered.reports.has_errors(), "{:?}", rendered.reports);

    // The query is left as written, and the answer appears beneath it.
    assert!(rendered.document.contains("```hql#eval"));
    assert!(rendered.document.contains("```hql#result"));
    assert!(rendered.document.contains("Bearer tokens"));
    assert!(rendered.document.contains("Session cookies"));
    // Everything that is done stays out of the list. (OAuth appears further
    // down, in the ranking, which is a different block.)
    let todo = rendered
        .document
        .split("## Closest")
        .next()
        .expect("the to-do section");
    assert!(!todo.contains("OAuth 2.0"), "{todo}");
}

#[test]
fn a_ranking_transcludes_as_a_table() {
    let rendered = transclude::render(&board(), &fixture(), Mode::Strict);
    assert!(rendered.document.contains("rank  score"));
    assert!(rendered.document.contains("bearer-tokens"));
}

#[test]
fn rendering_twice_leaves_what_rendering_once_did() {
    let vault = fixture();
    let once = transclude::render(&board(), &vault, Mode::Strict);
    let twice = transclude::render(&once.document, &vault, Mode::Strict);
    assert_eq!(once.document, twice.document);
    assert_eq!(twice.blocks, 2);
}

#[test]
fn a_block_that_fails_reports_and_the_document_still_renders() {
    let source = "# Notes\n\n```hql\ncards | nosuchstage\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict);
    assert_eq!(rendered.blocks, 1);
    assert!(rendered.reports.has_errors());
    // The document is still produced, with the failure where the answer goes.
    assert!(rendered.document.contains("```hql#result"));
    assert!(rendered.document.contains("error:"));
}

#[test]
fn a_fence_that_is_not_hql_is_left_alone() {
    let source = "```rust\nfn main() {}\n```\n";
    let rendered = transclude::render(source, &fixture(), Mode::Strict);
    assert_eq!(rendered.blocks, 0);
    assert_eq!(rendered.document, source);
}
