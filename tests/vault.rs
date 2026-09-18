//! The vault: loading documents, the card family, traversal and retrieval.

use hql::reporting::Mode;
use hql::types::Type;
use hql::values::Value;
use hql::{run, vault};
use std::path::Path;

fn fixture() -> vault::Vault {
    vault::load(Path::new("tests/fixtures/vault")).expect("the fixture vault loads")
}

fn value(source: &str) -> Value {
    let outcome = run(source, &fixture(), Mode::Strict);
    outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {:?}", outcome.reports))
}

fn text(source: &str) -> String {
    value(source).to_string()
}

#[test]
fn loads_markdown_and_hypermarkdown_alike() {
    let vault = fixture();
    let names: Vec<&str> = vault.cards.iter().map(|card| card.name()).collect();
    assert_eq!(
        names,
        [
            "bearer-tokens",
            "broken-relation",
            "http-headers",
            "oauth",
            "oauth-defines-bearer",
            "sessions",
        ]
    );
    // Card-ness is a knowledge notion, so a plain `.md` file is a card too.
    let markdown = vault.resolve("http-headers").expect("the Markdown card");
    assert_eq!(markdown.document.format.name(), "Markdown");
    assert_eq!(markdown.document.title(), "HTTP headers");
}

#[test]
fn a_declared_relation_is_checked_rather_than_trusted() {
    let vault = fixture();
    let verified = vault
        .resolve("oauth-defines-bearer")
        .expect("the relation card");
    assert_eq!(verified.kind, hql::document::Kind::Relation);

    // The endpoints do not resolve, so the claim fails and the card stays a
    // concept card — with the failed claim retained, not discarded.
    let unverified = vault.resolve("broken-relation").expect("the broken card");
    assert_eq!(unverified.kind, hql::document::Kind::Concept);
    assert!(
        unverified.metadata.path("knowledge.unverified").is_some(),
        "the failed claim is kept"
    );
    assert!(
        vault
            .warnings
            .iter()
            .any(|warning| warning.contains("broken-relation")),
        "loading said so"
    );
}

#[test]
fn typed_selects_the_kind_the_checker_established() {
    assert_eq!(text("cards | typed(RelationCard) | count"), "1");
    assert_eq!(text("cards | typed(ConceptCard) | count"), "5");
    assert_eq!(
        text("cards | typed(RelationCard) | map(c => c.name)"),
        "[oauth-defines-bearer]"
    );
}

#[test]
fn a_prefix_comes_from_the_cards_own_order_not_the_order_they_were_found() {
    // `cards` is a Set, and a Card is orderable by its name, so a prefix is
    // reproducible without the vault's discovery order being observable.
    assert_eq!(
        text("cards | take(2) | map(c => c.name)"),
        "[bearer-tokens, broken-relation]"
    );
    assert_eq!(
        text("cards | sort(by = c => c.title) | take(1) | map(c => c.title)"),
        "[A relation whose endpoints are missing]"
    );
}

#[test]
fn fields_reach_both_metadata_layers() {
    assert_eq!(text("[[bearer-tokens]].title"), "Bearer tokens");
    assert_eq!(text("[[bearer-tokens]].kind"), "ConceptCard");
    // The authored block and the assembled layer are different fields.
    assert_eq!(text("[[bearer-tokens]].header.metadata.status"), "\"todo\"");
    assert_eq!(text("[[bearer-tokens]].metadata.status"), "\"todo\"");
    // An extension contributes under a key it owns, so nothing collides.
    assert_eq!(
        text("[[bearer-tokens]].metadata.search.model"),
        "\"hql.hashbag.v1\""
    );
}

#[test]
fn uplinks_and_downlinks_traverse_one_edge_set_in_two_directions() {
    assert_eq!(text("[[bearer-tokens]] | uplinks | count"), "2");
    assert_eq!(text("[[bearer-tokens]] | downlinks | count"), "2");
    assert_eq!(
        text("[[http-headers]] | downlinks | map(e => e.source)"),
        "[bearer-tokens]"
    );
}

#[test]
fn semantic_ranks_by_similarity_and_retains_the_retrieval() {
    let ranked = value("import semantic\ncards | semantic(\"bearer token authorization\")");
    let Value::Ranking(ranking) = &ranked else {
        panic!("expected a ranking, got {ranked:?}")
    };
    // `oauth` writes every word of the query and `bearer-tokens` writes two
    // of them, so the order is decided by the articles rather than by the
    // filenames: an index sees what a document says, not where it sits.
    assert_eq!(ranking.hits[0].card.name(), "oauth");
    assert!(ranking.hits[0].score > ranking.hits[1].score);
    // A score is evidence, so it travels with the rule that produced it.
    assert_eq!(ranking.retrieval.model, hql::search::MODEL);
    assert_eq!(ranking.retrieval.metric, "cosine");
    assert!(!ranking.retrieval.approximate);
    assert_eq!(ranking.retrieval.query, "bearer token authorization");

    assert_eq!(
        text(
            "import semantic\ncards | semantic(\"bearer token authorization\") | take(1) | map(h => h.card.name)"
        ),
        "[oauth]"
    );
}

#[test]
fn an_unrelated_card_is_absent_from_a_ranking_rather_than_last_in_it() {
    let ranked = value("import semantic\ncards | semantic(\"bearer token authorization\")");
    let Value::Ranking(ranking) = &ranked else {
        panic!("expected a ranking")
    };
    let names: Vec<&str> = ranking.hits.iter().map(|hit| hit.card.name()).collect();
    assert!(!names.contains(&"sessions"), "{names:?}");
}

#[test]
fn the_whole_pipeline_runs_and_says_why_each_node_is_present() {
    let projected = value(
        "import semantic\n\
         cards\n\
         | semantic(\"bearer token authorization\")\n\
         | take(2)\n\
         | expand(depth = 1)\n\
         | graph",
    );
    let Value::Graph(graph) = &projected else {
        panic!("expected a graph, got {projected:?}")
    };
    assert!(graph.nodes.contains(&"bearer-tokens".to_owned()));
    // Expansion pulled in a node nobody scored, and the graph says so rather
    // than asserting that every node matched the query.
    assert!(matches!(
        graph.presence.get("bearer-tokens"),
        Some(hql::graph::Presence::Retrieved(_))
    ));
    assert!(matches!(
        graph.presence.get("http-headers"),
        Some(hql::graph::Presence::Expanded { .. })
    ));
}

#[test]
fn a_relation_card_contributes_an_edge_without_becoming_a_node() {
    let projected = value("cards | typed(RelationCard) | graph");
    let Value::Graph(graph) = &projected else {
        panic!("expected a graph")
    };
    assert!(
        !graph.nodes.contains(&"oauth-defines-bearer".to_owned()),
        "a relation card is not a node: {:?}",
        graph.nodes
    );
    assert!(graph.nodes.contains(&"oauth".to_owned()));
    assert!(graph.nodes.contains(&"bearer-tokens".to_owned()));
}

#[test]
fn an_unresolved_reference_warns_and_yields_absence() {
    let outcome = run("[[nowhere]]", &fixture(), Mode::Strict);
    assert_eq!(outcome.value, Some(Value::Absent(Type::Card)));
    assert!(!outcome.failed(), "a forward link is permitted");
    assert_eq!(outcome.reports.len(), 1);
    let report = outcome.reports.iter().next().expect("one warning");
    assert_eq!(report.severity, hql::reporting::Severity::Warning);
    assert!(report.message.contains("nowhere"));

    // Absence propagates through a field rather than becoming a failure.
    let outcome = run("[[nowhere]].title", &fixture(), Mode::Strict);
    assert_eq!(outcome.value, Some(Value::Absent(Type::Str)));
}

#[test]
fn the_expansion_direction_is_stated_rather_than_assumed() {
    let outcome = run(
        "[[http-headers]] | graph | expand(depth = 1, direction = \"uplinks\")",
        &fixture(),
        Mode::Strict,
    );
    let Some(Value::Graph(graph)) = &outcome.value else {
        panic!("expected a graph, got {:?}", outcome.reports)
    };
    // http-headers writes no references, so following only what leaves it
    // reaches nothing further.
    assert_eq!(graph.nodes, ["http-headers"]);

    let outcome = run(
        "[[http-headers]] | graph | expand(depth = 1, direction = \"downlinks\")",
        &fixture(),
        Mode::Strict,
    );
    let Some(Value::Graph(graph)) = &outcome.value else {
        panic!("expected a graph")
    };
    assert!(graph.nodes.contains(&"bearer-tokens".to_owned()));
}
