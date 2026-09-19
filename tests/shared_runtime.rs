//! Runtime carriers can cross thread boundaries without copying their contents.

use hql::{
    data::Data,
    document::Card,
    extensions::{Extension, Step},
    graph::Graph,
    search::{Hit, Ranking, Retrieval},
    types::Value,
    vault::Vault,
};

#[test]
fn runtime_and_extension_boundaries_are_send_and_sync() {
    fn shared<T: Send + Sync>() {}
    shared::<Value>();
    shared::<Card>();
    shared::<Data>();
    shared::<Graph>();
    shared::<Hit>();
    shared::<Ranking>();
    shared::<Retrieval>();
    shared::<Vault>();
    shared::<Step>();
    shared::<Extension>();
}

#[test]
fn concurrent_queries_share_a_vault_and_retrieval_provenance() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let outcome = hql::run(
                        "import lexical\ncards | lexical(\"bearer token\")",
                        &vault,
                        hql::reporting::Mode::Strict,
                    );
                    assert!(!outcome.failed(), "{:?}", outcome.reports);
                    let Some(Value::Ranking(ranking)) = outcome.value else {
                        panic!("expected ranking")
                    };
                    for hit in &ranking.hits {
                        assert!(std::sync::Arc::ptr_eq(&hit.provenance, &ranking.retrieval));
                        assert!(std::sync::Arc::ptr_eq(
                            &hit.card,
                            &vault.resolve(hit.card.name()).unwrap()
                        ));
                    }
                    ranking
                        .hits
                        .iter()
                        .map(|hit| (hit.card.name().to_owned(), hit.score))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    });
}
