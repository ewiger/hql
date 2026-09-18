//! The birds corpus: the vault retrieval is measured against.
//!
//! The corpus exists so that a lexical match and a semantic one can be told
//! apart. Its articles describe owls as nocturnal and never as hunting at
//! night, divers as pursuing prey beneath the surface and never as swimming
//! underwater, and weavers as plaiting a suspended pouch and never as building
//! a hanging nest. These tests pin the properties the retrieval tests rest on,
//! so a later edit cannot quietly dissolve the difference they measure.

use hql::document::Kind;
use hql::reporting::Mode;
use hql::types::Value;
use hql::{run, vault};
use std::path::Path;

const BIRDS: &str = "tests/fixtures/birds";

fn birds() -> vault::Vault {
    vault::load(Path::new(BIRDS)).expect("the birds vault loads")
}

fn text_of(source: &str) -> String {
    let outcome = run(source, &birds(), Mode::Strict);
    outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {:?}", outcome.reports))
        .to_string()
}

/// The words a query would share with an article, as the lexical matcher
/// tokenises them: anything longer than one character, lowercased.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() > 1)
        .map(str::to_lowercase)
        .collect()
}

#[test]
fn the_corpus_is_large_enough_and_holds_every_card_kind() {
    let vault = birds();
    assert!(vault.cards.len() >= 40, "{}", vault.cards.len());

    let relations: Vec<&str> = vault
        .cards
        .iter()
        .filter(|card| card.kind == Kind::Relation)
        .map(|card| card.name())
        .collect();
    assert_eq!(
        relations,
        [
            "barn-owl-in-tytonidae",
            "great-cormorant-in-phalacrocoracidae",
            "tytonidae-in-strigiformes",
            "village-weaver-in-ploceidae",
        ]
    );

    // An authored claim whose endpoints are not in the vault is retained and
    // does not hold, so the card stays a concept card.
    let unresolved = vault.resolve("condor-in-cathartidae").expect("the card");
    assert_eq!(unresolved.kind, Kind::Concept);

    let with_status = vault
        .cards
        .iter()
        .filter(|card| card.metadata.path("status").is_some())
        .count();
    assert!(with_status > 0 && with_status < vault.cards.len());
}

#[test]
fn a_forward_link_to_a_document_nobody_wrote_warns_rather_than_failing() {
    let outcome = run("[[andean-condor]]", &birds(), Mode::Strict);
    assert_eq!(outcome.value, Some(Value::Absent(hql::types::Type::Card)));
    assert!(
        outcome
            .reports
            .iter()
            .any(|report| report.message.contains("andean-condor")),
        "{:?}",
        outcome.reports
    );
}

#[test]
fn the_three_query_groups_share_no_words_with_the_queries_they_answer() {
    // This is the property the whole corpus exists for. Each group answers its
    // query and writes not one of the query's words, so a matcher over shared
    // spellings cannot find it and one over meaning can. The queries carry no
    // function words either, because `lexical` tokenises `that` and `at` like
    // any other word and a shared `that` is not evidence of anything.
    for (query, group) in [
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
            ][..],
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
            ][..],
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
            ][..],
        ),
    ] {
        let asked = words(query);
        for name in group {
            let card = birds().resolve(name).expect("a card of the group");
            let article = words(&card.document.text());
            for word in &asked {
                assert!(
                    !article.contains(word),
                    "{name} writes `{word}`, which `{query}` also writes"
                );
            }
        }
    }
}

#[test]
fn the_corpus_answers_its_queries_through_links_rather_than_isolated_cards() {
    // Every group card is reachable from its order or family, so a retrieval
    // that finds one card and expands can reach the rest.
    assert!(text_of("[[barn-owl]] | uplinks | count").trim() != "0");
    assert_eq!(text_of("[[tytonidae]] | downlinks | count").trim(), "3");
}
