//! The programs of the front page, run as the page runs them. The same
//! tests run natively with `cargo test -p hql-wasm` and in Node with
//! `wasm-pack test --node crates/hql-wasm`.

use hql_wasm::{check, eval, version};
use wasm_bindgen_test::wasm_bindgen_test;

const BOOKS: &str = r#"type Book { title : String, year : Int, rating : Int }

books = [
    Book(title = "Dune", year = 1965, rating = 5),
    Book(title = "Emma", year = 1815, rating = 3),
    Book(title = "Beloved", year = 1987, rating = 4),
]

books
| filter(b => b.rating >= 4)
| sort(b => b.title)
| map(b => b.title)
"#;

#[test]
#[wasm_bindgen_test]
fn the_query_gives_its_titles() {
    let report = eval(BOOKS);
    assert!(report.ok, "{}", report.diagnostics);
    assert_eq!(report.output, "[\"Beloved\", \"Dune\"]\n");
}

#[test]
#[wasm_bindgen_test]
fn the_contract_refuses_a_string_year() {
    let report = check(&BOOKS.replace("year = 1815", "year = \"1815\""));
    assert!(!report.ok);
    assert!(
        report.diagnostics.starts_with(
            "check error[type-mismatch]: `year` of `Book` is Int, and this is String\n \
             --> playground.hql:5:33\n"
        ),
        "{}",
        report.diagnostics
    );
}

#[test]
#[wasm_bindgen_test]
fn the_vault_is_the_reading_club() {
    let report = eval("cards | size");
    assert!(report.ok, "{}", report.diagnostics);
    assert_eq!(report.output, "15\n");

    let report = eval(
        "cards\n| filter(c => c.metadata.status == \"todo\")\n| sort(by = c => c.title)\n| \
         map(c => c.title)",
    );
    assert!(report.ok, "{}", report.diagnostics);
    assert_eq!(
        report.output,
        "[\"Leaves of Grass\", \"The Common Reader\", \"The Hobbit\"]\n"
    );

    let report = eval("@[[dune]].title");
    assert!(report.ok, "{}", report.diagnostics);
    assert_eq!(report.output, "\"Dune\"\n");
}

#[test]
#[wasm_bindgen_test]
fn a_server_endpoint_is_unavailable_in_a_page() {
    if cfg!(not(target_arch = "wasm32")) {
        // The native test build may be unified with `native` by the
        // workspace; the claim is the page's.
        return;
    }
    let report = eval("fetch(\"https://example.com/\")");
    assert!(!report.ok);
}

#[test]
#[wasm_bindgen_test]
fn the_clock_and_the_seed_are_the_pages() {
    let report = eval("from time import now\nnow()");
    assert!(report.ok, "{}", report.diagnostics);
    let report = eval("from random import *\nrandom_int(1, 6) >= 1");
    assert!(report.ok, "{}", report.diagnostics);
}

#[test]
#[wasm_bindgen_test]
fn the_version_is_the_crates() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
