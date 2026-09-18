//! Retrieval over a precomputed embedding index.
//!
//! Every test here runs against the committed index and needs no Python: the
//! producer is a separate tool, and a contributor without it still builds and
//! tests everything.

use hql::reporting::Mode;
use hql::types::Value;
use hql::{run, vault};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};

const BIRDS: &str = "tests/fixtures/birds";

/// The three queries the corpus was written to answer, each with the cards
/// that answer it. No article writes a word of the query it answers.
const PAIRS: &[(&str, &[&str])] = &[
    (
        "night hunting birds",
        &[
            "strigiformes",
            "tytonidae",
            "strigidae",
            "barn-owl",
            "tawny-owl",
            "great-horned-owl",
            "little-owl",
            "snowy-owl",
        ],
    ),
    (
        "birds swimming underwater catching fish",
        &[
            "phalacrocoracidae",
            "great-cormorant",
            "gaviidae",
            "common-loon",
            "spheniscidae",
            "gentoo-penguin",
            "emperor-penguin",
            "alcidae",
            "razorbill",
        ],
    ),
    (
        "birds weaving hanging nests",
        &[
            "ploceidae",
            "village-weaver",
            "icteridae",
            "baltimore-oriole",
            "remizidae",
            "penduline-tit",
        ],
    ),
];

fn birds() -> vault::Vault {
    vault::load(Path::new(BIRDS)).expect("the birds vault loads")
}

fn top_three(step: &str, query: &str, vault: &vault::Vault) -> Vec<String> {
    let source =
        format!("import {step}\ncards | {step}(\"{query}\") | take(3) | map(h => h.card.name)");
    let outcome = run(&source, vault, Mode::Collect);
    let value = outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {:?}", outcome.reports));
    let Value::Seq(elements, _) = &value else {
        panic!("expected a sequence, got {value:?}")
    };
    elements
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

/// A writable copy of the birds vault, so a test may spoil its index.
fn copied() -> (TempDir, PathBuf) {
    let directory = tempdir().expect("a temporary directory");
    let root = directory.path().join("birds");
    fs::create_dir_all(root.join(".hql")).expect("the vault directory");
    for entry in fs::read_dir(BIRDS).expect("the fixture vault") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            fs::copy(&path, root.join(path.file_name().expect("a name"))).expect("a copy");
        }
    }
    fs::copy(
        Path::new(BIRDS).join(".hql/index.sqlite"),
        root.join(".hql/index.sqlite"),
    )
    .expect("the index");
    (directory, root)
}

fn spoil(root: &Path, statements: &str) {
    let connection =
        rusqlite::Connection::open(root.join(".hql/index.sqlite")).expect("the copied index");
    connection.execute_batch(statements).expect("the change");
}

fn failure(root: &Path, source: &str) -> String {
    let vault = vault::load(root).expect("the copied vault loads");
    let outcome = run(source, &vault, Mode::Collect);
    outcome
        .reports
        .iter()
        .map(|report| report.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_paraphrase_finds_the_cards_that_answer_it_and_shared_spellings_do_not() {
    // The whole point of the corpus and of the split. `semantic` answers all
    // three questions; `lexical` cannot see any of them, because not one of the
    // articles that answer a query writes a word of it.
    let vault = birds();
    for (query, group) in PAIRS {
        let found = top_three("semantic", query, &vault);
        assert_eq!(found.len(), 3, "{query}");
        for name in &found {
            assert!(
                group.contains(&name.as_str()),
                "{query}: {name} is not one of the group"
            );
        }

        let missed = top_three("lexical", query, &vault);
        for name in &missed {
            assert!(
                !group.contains(&name.as_str()),
                "{query}: `lexical` found {name}, which it has no word to find it by"
            );
        }
    }
}

#[test]
fn a_score_travels_with_the_model_that_produced_it() {
    let vault = birds();
    let outcome = run(
        "import semantic\ncards | semantic(\"night hunting birds\")",
        &vault,
        Mode::Strict,
    );
    let Some(Value::Ranking(ranking)) = &outcome.value else {
        panic!("expected a ranking, got {:?}", outcome.reports)
    };
    let retrieval = &ranking.retrieval;
    assert_eq!(retrieval.model, "sentence-transformers/all-MiniLM-L6-v2");
    // A model's name does not pin the numbers a score is; its revision does.
    assert_eq!(retrieval.revision.len(), 40);
    assert_eq!(retrieval.metric, "cosine");
    assert!(!retrieval.approximate, "scoring is exact brute force");
    assert!(retrieval.index.ends_with("index.sqlite"));
    assert_eq!(retrieval.query, "night hunting birds");
    assert!(!outcome.failed(), "{:?}", outcome.reports);
}

#[test]
fn the_index_agrees_with_this_binary_about_what_a_card_says() {
    // The golden test, from the Rust side and without Python: every vector was
    // computed from the text this binary hashes today. A disagreement has no
    // other symptom — the index would score text nobody wrote.
    let vault = birds();
    let outcome = run(
        "import semantic\ncards | semantic(\"night hunting birds\") | count",
        &vault,
        Mode::Collect,
    );
    assert_eq!(outcome.value, Some(Value::Int(56)));
    assert!(
        outcome.reports.iter().count() == 0,
        "a stale or missing card would warn here: {:?}",
        outcome.reports
    );
}

#[test]
fn a_card_the_index_has_not_seen_is_absent_from_the_ranking_and_counted() {
    let (_directory, root) = copied();
    fs::write(
        root.join("kakapo.hmd"),
        "---\ntitle: Kakapo\n---\n# Kakapo\n\nA flightless nocturnal parrot of New Zealand.\n",
    )
    .expect("a new card");
    let vault = vault::load(&root).expect("the copied vault loads");
    let outcome = run(
        "import semantic\ncards | semantic(\"night hunting birds\") | count",
        &vault,
        Mode::Collect,
    );
    // Absent from the index is absent from the ranking: a zero score would be
    // indistinguishable from indexed and unrelated.
    assert_eq!(outcome.value, Some(Value::Int(56)));
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(said.contains("1 card is absent from the index"), "{said}");
}

#[test]
fn a_card_edited_since_the_build_still_ranks_and_says_it_is_stale() {
    let (_directory, root) = copied();
    let path = root.join("barn-owl.hmd");
    let edited = fs::read_to_string(&path).expect("the card") + "\nAn added sentence.\n";
    fs::write(&path, edited).expect("the edit");
    let vault = vault::load(&root).expect("the copied vault loads");
    let outcome = run(
        "import semantic\ncards | semantic(\"night hunting birds\") | count",
        &vault,
        Mode::Collect,
    );
    // A stale score is a wrong answer, not an impossible one, so it ranks.
    assert_eq!(outcome.value, Some(Value::Int(56)));
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(
        said.contains("`barn-owl` has changed since the index was built"),
        "{said}"
    );
    assert!(said.contains("hql-semantics build"), "{said}");
}

#[test]
fn a_schema_the_reader_does_not_know_is_refused() {
    let (_directory, root) = copied();
    spoil(&root, "UPDATE index_meta SET schema_version = 2");
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(said.contains("unknown index schema"), "{said}");
}

#[test]
fn a_vector_that_is_not_the_declared_length_is_refused_rather_than_read_past() {
    let (_directory, root) = copied();
    spoil(
        &root,
        "UPDATE card SET embedding = substr(embedding, 1, 40) WHERE name = 'barn-owl'",
    );
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(said.contains("`barn-owl` has a 40-byte vector"), "{said}");
}

#[test]
fn an_index_built_for_another_corpus_is_refused() {
    let (_directory, root) = copied();
    spoil(&root, "UPDATE index_meta SET vault = 'somewhere/else'");
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(said.contains("was built for `somewhere/else`"), "{said}");
}

#[test]
fn a_query_the_index_has_no_vector_for_says_so_rather_than_inventing_one() {
    // This binary has no model. A query it was not given a vector for is a
    // failure that names the command, not a ranking of nothing.
    let vault = birds();
    let outcome = run(
        "import semantic\ncards | semantic(\"a question nobody embedded\")",
        &vault,
        Mode::Collect,
    );
    let said: String = outcome
        .reports
        .iter()
        .map(|report| report.message.clone())
        .collect();
    assert!(said.contains("holds no vector"), "{said}");
    assert!(said.contains("--query"), "{said}");
}

#[test]
fn a_vault_with_no_index_is_told_to_build_one_rather_than_falling_back() {
    // `semantic` never falls back to `lexical`: a silent fallback restores the
    // confusion that renaming the matcher removed.
    let vault = vault::load(Path::new("tests/fixtures/vault")).expect("the fixture vault");
    let outcome = run(
        "import semantic\ncards | semantic(\"bearer token authorization\")",
        &vault,
        Mode::Collect,
    );
    let said: String = outcome
        .reports
        .iter()
        .map(|report| report.message.clone())
        .collect();
    assert!(said.contains("[semantics] index"), "{said}");
    assert!(said.contains("hql-semantics build"), "{said}");
    assert_eq!(outcome.value, None);
}

#[test]
fn an_index_outside_the_vault_is_refused() {
    let (_directory, root) = copied();
    fs::write(
        root.join("hql.toml"),
        "[semantics]\nindex = \"../escaped.sqlite\"\n",
    )
    .expect("the configuration");
    let said = failure(
        &root,
        "import semantic\ncards | semantic(\"night hunting birds\")",
    );
    assert!(said.contains("outside the vault"), "{said}");
}
