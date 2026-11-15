//! The vault: loading documents, the card family, traversal and retrieval.

use hql::reporting::Mode;
use hql::types::TypeRef;
use hql::types::Value;
use hql::{run, vault};
use std::path::Path;

fn fixture() -> hql::environment::RunEnvironment {
    hql::environment::load(Path::new("tests/fixtures/vault")).expect("the fixture vault loads")
}

fn value(source: &str) -> Value {
    let outcome = run(source, &fixture(), Mode::Strict);
    outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {:?}", outcome.diagnostics))
}

fn text(source: &str) -> String {
    value(source).to_string()
}

#[test]
fn loads_markdown_and_hypermarkdown_alike() {
    let vault = fixture();
    let names: Vec<&str> = vault.cards().iter().map(|card| card.name()).collect();
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
        vault.warnings().iter().any(|warning| warning
            .origin
            .as_deref()
            .is_some_and(|origin| origin.contains("broken-relation"))),
        "loading said so"
    );
}

#[test]
fn of_type_selects_the_kind_the_checker_established() {
    assert_eq!(text("cards | of_type(RelationCard) | count"), "1");
    assert_eq!(text("cards | of_type(ConceptCard) | count"), "5");
    assert_eq!(
        text("cards | of_type(RelationCard) | map(c => c.name)"),
        "[\"oauth-defines-bearer\"]"
    );
}

#[test]
fn of_type_selects_the_dialect_of_a_cards_body() {
    assert_eq!(
        text("import doc\ncards | of_type(MdCard) | map(c => c.name)"),
        "[\"http-headers\"]"
    );
    assert_eq!(text("import doc\ncards | of_type(HmdCard) | count"), "5");
    assert_eq!(text("import doc\ncards | of_type(HmdDoc) | count"), "5");
    // A card's kind and its dialect are independent.
    assert_eq!(
        text("import doc\ncards | of_type(RelationCard) | of_type(HmdCard) | count"),
        "1"
    );
    assert_eq!(
        text("import doc\ncards | filter(c => c | is(MdCard)) | count"),
        "1"
    );
    let vault = fixture();
    let refused = hql::check_in(
        "import doc\ncards | of_type(HmdCard) | of_type(MdCard)",
        &vault,
    )
    .expect_err("two dialects exclude each other");
    assert!(refused.message().contains("no HmdCard can be a MdCard"));
}

#[test]
fn a_programs_own_card_subtype_is_refused_rather_than_silently_empty() {
    let vault = fixture();
    let declared = "type Task { status : String }\ntype TaskCard : Card { metadata : Task }\n";
    for test in [
        "cards | of_type(TaskCard)",
        "cards | filter(c => c | is(TaskCard))",
    ] {
        let refused = hql::check_in(&format!("{declared}{test}"), &vault)
            .expect_err("a program's card subtype");
        assert!(
            refused.message().contains("no card is a TaskCard")
                && refused.message().contains("c.metadata | T"),
            "{test}: {}",
            refused.message()
        );
    }
}

#[test]
fn a_filtered_card_pipeline_needs_no_annotations_or_conversions() {
    let vault = fixture();
    let binding = "open = cards | filter(c => c.metadata.status == \"todo\")\n";
    for (expression, expected) in [
        // `cards` is a sequence, so sorting keeps it one.
        ("open", TypeRef::seq(TypeRef::CARD)),
        (
            "open | sort(by = c => c.title)",
            TypeRef::seq(TypeRef::CARD),
        ),
        (
            "open | sort(by = c => c.title) | map(c => c.title)",
            TypeRef::seq(TypeRef::STR),
        ),
        (
            "open | sort(by = c => c.title) | map(c => c.title) | markdown",
            TypeRef::STR,
        ),
    ] {
        assert_eq!(
            hql::check_in(&format!("{binding}{expression}"), &vault),
            Ok(expected)
        );
    }
    assert_eq!(
        text(&format!(
            "{binding}open | sort(by = c => c.title) | map(c => c.title)"
        )),
        "[\"Bearer tokens\", \"Session cookies\"]"
    );
}

#[test]
fn a_prefix_comes_from_the_cards_own_order_not_the_order_they_were_found() {
    // `cards` is a Set, and a Card is orderable by its name, so a prefix is
    // reproducible without the vault's discovery order being observable.
    assert_eq!(
        text("cards | take(2) | map(c => c.name)"),
        "[\"bearer-tokens\", \"broken-relation\"]"
    );
    assert_eq!(
        text("cards | sort(by = c => c.title) | take(1) | map(c => c.title)"),
        "[\"A relation whose endpoints are missing\"]"
    );
}

#[test]
fn fields_reach_both_metadata_layers() {
    assert_eq!(text("@[[bearer-tokens]].title"), "Bearer tokens");
    assert_eq!(text("@[[bearer-tokens]].kind"), "ConceptCard");
    // The authored block and the assembled layer are different fields.
    assert_eq!(
        text("@[[bearer-tokens]].header.metadata.status"),
        "\"todo\""
    );
    assert_eq!(text("@[[bearer-tokens]].metadata.status"), "\"todo\"");
    // An extension contributes under a key it owns, so nothing collides.
    assert_eq!(
        text("@[[bearer-tokens]].metadata.lexical.model"),
        "\"hql.hashbag.v1\""
    );
}

#[test]
fn uplinks_and_downlinks_traverse_one_edge_set_in_two_directions() {
    assert_eq!(text("@[[bearer-tokens]] | uplinks | count"), "2");
    assert_eq!(text("@[[bearer-tokens]] | downlinks | count"), "2");
    assert_eq!(
        text("@[[http-headers]] | downlinks | map(e => e.source)"),
        "[\"bearer-tokens\"]"
    );
}

#[test]
fn lexical_ranks_by_shared_words_and_retains_the_retrieval() {
    let ranked = value("import hmd\ncards | lexical(\"bearer token authorization\")");
    let Value::Ranking(ranking) = &ranked else {
        panic!("expected a ranking, got {ranked:?}")
    };
    // `oauth` writes every word of the query and `bearer-tokens` writes two
    // of them, so the order is decided by the articles rather than by the
    // filenames: an index sees what a document says, not where it sits.
    assert_eq!(ranking.hits[0].name(), "oauth");
    assert!(ranking.hits[0].score > ranking.hits[1].score);
    // A score is evidence, so it travels with the rule that produced it.
    assert_eq!(ranking.retrieval.model, hql::extensions::lexical::MODEL);
    assert_eq!(ranking.retrieval.metric, "cosine");
    assert!(!ranking.retrieval.approximate);
    assert_eq!(ranking.retrieval.query, "bearer token authorization");

    assert_eq!(
        text(
            "import hmd\ncards | lexical(\"bearer token authorization\") | take(1) | map(h => h.value.name)"
        ),
        "[\"oauth\"]"
    );
}

#[test]
fn a_new_order_is_not_the_retrieval_order_so_a_sorted_ranking_is_a_sequence() {
    let query = "import hmd\nranked = cards | lexical(\"bearer token authorization\")\n";
    let hits = TypeRef::hit(TypeRef::CARD);
    assert_eq!(
        hql::check_in(&format!("{query}ranked | take(1)"), &fixture()),
        Ok(TypeRef::ranking(TypeRef::CARD))
    );
    assert_eq!(
        hql::check_in(
            &format!("{query}ranked | sort(by = h => h.value.name)"),
            &fixture()
        ),
        Ok(TypeRef::seq(hits))
    );
    assert_eq!(
        hql::check_in(
            &format!("{query}ranked | map(h => h.value.name)"),
            &fixture()
        ),
        Ok(TypeRef::seq(TypeRef::STR))
    );
}

#[test]
fn is_tests_a_type_and_a_filter_by_it_narrows_the_element_type() {
    let vault = fixture();
    let relation = TypeRef::RELATION_CARD;
    assert_eq!(
        hql::check_in("cards | filter(c => c | is(RelationCard))", &vault),
        Ok(TypeRef::seq(relation.clone()))
    );
    assert_eq!(
        hql::check_in("cards | filter(c => is(c, RelationCard))", &vault),
        Ok(TypeRef::seq(relation.clone()))
    );
    assert_eq!(
        hql::check_in("cards | filter(c => c.name == \"x\")", &vault),
        Ok(TypeRef::seq(TypeRef::CARD))
    );
    assert_eq!(
        hql::check_in("cards | of_type(RelationCard)", &vault),
        Ok(TypeRef::seq(relation))
    );
    let narrowed = value("cards | filter(c => c | is(RelationCard))");
    assert!(narrowed.type_of().is(&TypeRef::seq(TypeRef::RELATION_CARD)));
    assert_eq!(
        text("cards | filter(c => c | is(RelationCard)) | count"),
        text("cards | of_type(RelationCard) | count")
    );
    assert_eq!(
        text("cards | sort | map(c => c | is(ConceptCard)) | count(true)"),
        "5"
    );
}

#[test]
fn a_test_no_value_can_pass_is_refused() {
    let vault = fixture();
    let always_false = hql::check_in("1 | is(Card)", &vault).unwrap_err();
    assert!(
        always_false.to_string().contains("always false"),
        "{always_false}"
    );
    let renamed = hql::check_in(
        "import hmd\ncards | lexical(\"owl\") | map(h => h.card)",
        &vault,
    )
    .unwrap_err();
    assert!(renamed.to_string().contains("`value`"), "{renamed}");
}

#[test]
fn an_unrelated_card_is_absent_from_a_ranking_rather_than_last_in_it() {
    let ranked = value("import hmd\ncards | lexical(\"bearer token authorization\")");
    let Value::Ranking(ranking) = &ranked else {
        panic!("expected a ranking")
    };
    let names: Vec<String> = ranking.hits.iter().map(search_name).collect();
    assert!(!names.iter().any(|name| name == "sessions"), "{names:?}");
}

#[test]
fn the_whole_pipeline_runs_and_says_why_each_node_is_present() {
    let projected = value(
        "import hmd\n\
         cards\n\
         | lexical(\"bearer token authorization\")\n\
         | take(2)\n\
         | expand(depth = 1)\n\
         | Graph",
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
    let projected = value("cards | of_type(RelationCard) | Graph");
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
    let outcome = run("@[[nowhere]]", &fixture(), Mode::Strict);
    assert_eq!(outcome.value, Some(Value::Absent(TypeRef::CARD)));
    assert!(!outcome.failed(), "a forward link is permitted");
    // What the loader wrote about the fixture comes first; the run's own
    // warning is the one execute wrote.
    let report = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.phase == hql::diagnostics::Phase::Execute)
        .expect("one warning");
    assert_eq!(report.severity, hql::reporting::Severity::Warning);
    assert!(report.message.contains("nowhere"));

    // Absence propagates through a field rather than becoming a failure.
    let outcome = run("@[[nowhere]].title", &fixture(), Mode::Strict);
    assert_eq!(outcome.value, Some(Value::Absent(TypeRef::STR)));
}

#[test]
fn the_expansion_direction_is_stated_rather_than_assumed() {
    let outcome = run(
        "@[[http-headers]] | Graph | expand(depth = 1, direction = \"uplinks\")",
        &fixture(),
        Mode::Strict,
    );
    let Some(Value::Graph(graph)) = &outcome.value else {
        panic!("expected a graph, got {:?}", outcome.diagnostics)
    };
    // http-headers writes no references, so following only what leaves it
    // reaches nothing further.
    assert_eq!(graph.nodes, ["http-headers"]);

    let outcome = run(
        "@[[http-headers]] | Graph | expand(depth = 1, direction = \"downlinks\")",
        &fixture(),
        Mode::Strict,
    );
    let Some(Value::Graph(graph)) = &outcome.value else {
        panic!("expected a graph")
    };
    assert!(graph.nodes.contains(&"bearer-tokens".to_owned()));
}

#[test]
fn cards_is_a_binding_a_program_may_shadow() {
    assert_eq!(text("cards | count"), "6");
    // Once the keyword is gone, the binding on the first line is what the
    // second reads.
    assert_eq!(text("cards = [1, 2]\ncards | count"), "2");
    assert_eq!(text("all = cards\nall | count"), "6");
}

#[test]
fn scalar_operators_filter_cards() {
    assert_eq!(
        text(
            "cards | filter(c => c.name < \"h\" and not c.name == \"broken-relation\") | map(c => c.name)"
        ),
        "[\"bearer-tokens\"]"
    );
}

fn search_name(hit: &hql::search::Hit) -> String {
    hit.name()
}

#[test]
fn cards_is_the_vaults_cards_in_the_order_a_sorted_walk_reaches_them() {
    let vault = fixture();
    assert_eq!(
        hql::check_in("cards", &vault),
        Ok(TypeRef::seq(TypeRef::CARD))
    );
    // A directory's entries follow it in name order, depth first, so the
    // card inside `protocols/` comes between `oauth` and `sessions`.
    assert_eq!(
        text("cards | map(c => c.name)"),
        "[\"bearer-tokens\", \"broken-relation\", \"oauth-defines-bearer\", \"oauth\", \"http-headers\", \"sessions\"]"
    );
    assert_eq!(
        text("vault.cards | take(2) | map(c => c.name)"),
        "[\"bearer-tokens\", \"broken-relation\"]"
    );
    // `cards` is an ordinary binding: a program may shadow it.
    assert_eq!(text("cards = [1, 2]\ncards | sum"), "3");
}

#[test]
fn without_a_vault_there_is_no_vault_and_no_cards() {
    let failure = hql::check("vault.cards").expect_err("no hmd");
    assert!(
        failure.message().contains("`vault` is HyperMarkdown's"),
        "{}",
        failure.message()
    );
    let failure = hql::check("import hmd\nvault.cards").expect_err("no vault");
    assert!(
        failure.message().contains("`vault` needs a vault"),
        "{}",
        failure.message()
    );
    let failure = hql::check("import hmd\ncards").expect_err("no vault");
    assert!(
        failure.message().contains("--vault"),
        "{}",
        failure.message()
    );
}

#[test]
fn a_graph_exposes_its_nodes_as_cards_and_its_edges_as_sets() {
    let vault = fixture();
    assert_eq!(
        hql::check_in("(cards | Graph).nodes", &vault),
        Ok(TypeRef::set(TypeRef::CARD))
    );
    assert_eq!(
        hql::check_in("(cards | Graph).edges", &vault),
        Ok(TypeRef::set(TypeRef::EDGE))
    );
    // The relation card is an edge, not a node.
    assert_eq!(text("g = cards | Graph\ng.nodes | size"), "5");
    assert_eq!(
        text("g = cards | Graph\ng.nodes | map(c => c.name) | sort"),
        "[\"bearer-tokens\", \"broken-relation\", \"http-headers\", \"oauth\", \"sessions\"]"
    );
}

#[test]
fn filter_over_one_optional_value_keeps_it_or_makes_it_absent() {
    let vault = fixture();
    for (source, expected) in [
        ("some(1) | filter(x => x > 0)", "1"),
        ("some(0) | filter(x => x > 0)", "none"),
        ("none | filter(x => x > 0)", "none"),
    ] {
        assert_eq!(text(source), expected, "{source}");
    }
    assert_eq!(
        hql::check_in("some(1) | filter(x => x > 0)", &vault),
        Ok(TypeRef::optional(TypeRef::INT))
    );
    // A narrowing predicate narrows the optional value.
    assert_eq!(
        hql::check_in(
            "@[[oauth-defines-bearer]] | filter(c => c | is(RelationCard))",
            &vault
        ),
        Ok(TypeRef::optional(TypeRef::RELATION_CARD))
    );
}

#[test]
fn a_bare_constructor_is_tested_whatever_its_arguments_are() {
    assert_eq!(text("[1] | is(List)"), "true");
    let refused = hql::check("{1} | is(List)").expect_err("a set is never a list");
    assert!(
        refused.message().contains("always false"),
        "{}",
        refused.message()
    );
}

#[test]
fn a_refused_annotation_says_what_to_write_instead() {
    let vault = fixture();
    let set = hql::check("xs : List<Int> = {1, 2}\nxs").expect_err("no conversion");
    assert!(set.message().contains("sort it"), "{}", set.message());
    let narrower = hql::check_in("r : Seq<RelationCard> = cards\nr", &vault).expect_err("narrower");
    assert!(
        narrower.message().contains("of_type(RelationCard)"),
        "{}",
        narrower.message()
    );
}

#[test]
fn a_graph_is_built_from_cards_from_edges_or_from_both() {
    let vault = fixture();
    assert_eq!(
        hql::check_in("@[[oauth]] | uplinks | Graph", &vault),
        Ok(TypeRef::GRAPH)
    );
    assert_eq!(
        text("(@[[oauth]] | uplinks | Graph).nodes | map(c => c.name) | sort"),
        "[\"bearer-tokens\", \"oauth\"]"
    );
    assert_eq!(
        text(
            "Graph(cards | filter(c => c.name == \"sessions\"), @[[oauth]] | uplinks).nodes | map(c => c.name) | sort"
        ),
        "[\"bearer-tokens\", \"oauth\", \"sessions\"]"
    );
    let wrong = hql::check_in("Graph(cards, [1])", &vault).expect_err("not edges");
    assert!(
        wrong.message().contains("edges are edges"),
        "{}",
        wrong.message()
    );
}

#[test]
fn the_vault_is_a_value_with_its_name_its_root_and_its_graph() {
    assert_eq!(text("vault.name"), "vault");
    assert!(text("vault.root").ends_with("tests/fixtures/vault"));
    assert_eq!(text("graph.nodes | size"), "5");
    assert_eq!(text("vault.graph.edges | size"), text("graph.edges | size"));
}

#[test]
fn discovery_stops_at_the_nearest_marked_directory() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("vault");
    let deep = root.join("notes/2026");
    std::fs::create_dir_all(&deep).unwrap();
    assert_eq!(vault::discover(&deep), None);

    std::fs::create_dir(root.join(".hmd")).unwrap();
    std::fs::write(root.join(".hmd/config.toml"), "wiki = \".\"\n").unwrap();
    assert_eq!(vault::discover(&deep), Some(root.clone()));
    assert_eq!(vault::discover(&root), Some(root.clone()));

    // A project file marks a project, not a vault: the walk passes it.
    let nested = root.join("notes");
    std::fs::create_dir_all(nested.join(".hql")).unwrap();
    std::fs::write(nested.join(".hql/config.toml"), "").unwrap();
    assert_eq!(vault::discover(&deep), Some(root.clone()));
    std::fs::create_dir(nested.join(".hmd")).unwrap();
    std::fs::write(nested.join(".hmd/config.toml"), "wiki = \".\"\n").unwrap();
    assert_eq!(vault::discover(&deep), Some(nested));
}

#[test]
fn a_nested_vault_is_not_part_of_the_vault_around_it() {
    let root = tempfile::tempdir().unwrap();
    let outer = root.path();
    std::fs::create_dir_all(outer.join(".hmd")).unwrap();
    std::fs::write(outer.join(".hmd/config.toml"), "wiki = \".\"\n").unwrap();
    std::fs::create_dir_all(outer.join("site/.hmd")).unwrap();
    std::fs::write(outer.join("site/.hmd/config.toml"), "wiki = \".\"\n").unwrap();
    std::fs::create_dir_all(outer.join("site/learn")).unwrap();
    std::fs::create_dir_all(outer.join("wiki")).unwrap();
    std::fs::write(outer.join("wiki/time.hmd"), "# Time").unwrap();
    std::fs::write(outer.join("site/index.hmd"), "# Site").unwrap();
    std::fs::write(outer.join("site/learn/time.hmd"), "# Time, taught").unwrap();

    let names = |vault: &hql::environment::RunEnvironment| {
        vault
            .cards()
            .iter()
            .map(|card| card.name().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&hql::environment::load(outer).unwrap()), ["time"]);
    assert_eq!(
        names(&hql::environment::load(&outer.join("site")).unwrap()),
        ["index", "time"]
    );
}

#[test]
fn the_cards_start_where_wiki_says_and_their_paths_are_relative_to_it() {
    // HQL issue 0079: a vault's cards are the documents below its namespace
    // root, and a card's path is written against that root.
    let root = tempfile::tempdir().unwrap();
    let vault = root.path();
    std::fs::create_dir_all(vault.join(".hmd")).unwrap();
    std::fs::write(vault.join(".hmd/config.toml"), "wiki = \"doc/wiki\"\n").unwrap();
    std::fs::create_dir_all(vault.join("doc/wiki/birds")).unwrap();
    std::fs::write(vault.join("README.md"), "# Readme").unwrap();
    std::fs::write(vault.join("doc/wiki/birds/owl.hmd"), "# Owl").unwrap();
    std::fs::write(vault.join("doc/wiki/heron.md"), "# Heron").unwrap();

    let loaded = vault::load_documents(vault).unwrap();
    let paths: Vec<_> = loaded
        .cards
        .iter()
        .map(|card| card.document.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(paths, ["heron.md", "birds/owl.hmd"]);
    assert_eq!(loaded.root, vault);
    assert_eq!(
        loaded.file_of(&loaded.cards[1].document),
        vault.join("doc/wiki/birds/owl.hmd")
    );
}

#[test]
fn a_vault_whose_cards_location_is_missing_names_the_setting() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join(".hmd")).unwrap();
    let error = vault::load_documents(root.path()).unwrap_err().to_string();
    assert!(
        error.contains("doc/wiki does not exist; set `wiki`"),
        "{error}"
    );
}

/// A vault written out in a temporary directory, its cards at the root.
fn written(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join(".hmd")).unwrap();
    std::fs::write(root.path().join(".hmd/config.toml"), "wiki = \".\"\n").unwrap();
    for (path, text) in files {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// Where each reference of the card named `source` lands, by card name.
fn targets(vault: &vault::Vault, source: &str) -> Vec<String> {
    vault
        .resolve(source)
        .unwrap()
        .document
        .references
        .iter()
        .map(|reference| reference.target.clone())
        .collect()
}

#[test]
fn a_reference_resolves_as_hypermarkdown_resolves_it() {
    // Issue 0079: HQL and HyperMarkdown name the same card for every
    // reference. Each case below is one the stem rule answered differently.
    let root = written(&[
        // The spine: the nearer `tokens` wins, though two cards share the stem.
        ("specs/login.hmd", "[[tokens]] [[tokens#Rotation]]"),
        ("specs/tokens.hmd", "# Tokens\n\n## Rotation"),
        ("shared/tokens.hmd", "# Shared tokens"),
        // A named import binds a name the spine does not hold.
        (
            "specs/api.hmd",
            "---\nimport: from /shared import tokens as shared-tokens\n---\n[[shared-tokens]]",
        ),
        // `.hmd` wins a name a `.md` file shares; `[[owl.md]]` reaches the other.
        ("birds.hmd", "[[owl]] [[owl.md]] [[nobody]]"),
        ("owl.hmd", "# Owl"),
        ("owl.md", "# Owl, the old notes"),
    ]);
    let vault = vault::load_documents(root.path()).unwrap();

    assert_eq!(targets(&vault, "login"), ["specs/tokens", "specs/tokens"]);
    assert_eq!(targets(&vault, "api"), ["shared/tokens"]);
    // The shadowed `.md` file keeps its extension in its name.
    assert!(vault.resolve("owl.md").is_some());
    assert_eq!(targets(&vault, "birds"), ["owl", "owl.md", "nobody"]);
}

#[test]
fn a_reference_in_a_program_resolves_from_the_namespace_root() {
    let root = written(&[
        ("a/x.hmd", "# X in a"),
        ("b/x.hmd", "# X in b"),
        ("c/lone.md", "# Lone"),
    ]);
    let vault = vault::load_documents(root.path()).unwrap();

    assert_eq!(vault.reference("lone").unwrap().unwrap().name(), "lone");
    assert_eq!(vault.reference("a/x").unwrap().unwrap().name(), "a/x");
    // Two cards end in `x`: HyperMarkdown chooses none, and neither does HQL.
    assert_eq!(vault.reference("x").unwrap_err(), ["a/x.hmd", "b/x.hmd"]);
    assert!(vault.reference("missing").unwrap().is_none());
}
