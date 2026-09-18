//! What a retrieval is: a score, the rule that produced it, and the ordering
//! the two together justify.
//!
//! Nothing here computes a score. An extension supplies the scores and this
//! module assembles them into a ranking, which is the split the knowledge model
//! asks for: an index contributes evidence, and what evidence means is not the
//! index's to decide.

use crate::document::Card;
use std::rc::Rc;

/// The rule a ranking was produced by, retained so a score stays evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retrieval {
    /// What was asked.
    pub query: String,
    /// Which corpus answered.
    pub index: String,
    /// Which model produced the vectors.
    pub model: String,
    /// Which revision of that model, because a model name alone does not pin
    /// the numbers a score is.
    pub revision: String,
    /// How vectors were compared.
    pub metric: String,
    /// Whether the search may have missed a better match.
    pub approximate: bool,
}

/// A card, its score, and the retrieval behind both.
#[derive(Debug, Clone)]
pub struct Hit {
    /// What was retrieved.
    pub card: Rc<Card>,
    /// The metric result, relative to one query.
    pub score: f64,
    /// The retained rule.
    pub provenance: Rc<Retrieval>,
}

/// An ordered collection of hits, with the query its order is relative to.
#[derive(Debug, Clone)]
pub struct Ranking {
    /// The hits, best first.
    pub hits: Vec<Hit>,
    /// The retained rule, shared by every hit.
    pub retrieval: Rc<Retrieval>,
}

/// Assemble scored cards into a ranking.
///
/// A card scoring nothing at all is dropped: zero similarity is not a weak
/// answer, it is the absence of one. Ties break on the card's own ordering
/// key, so a prefix never depends on the order a vault happened to load in.
#[must_use]
pub fn rank(scored: Vec<(Rc<Card>, f64)>, retrieval: Retrieval) -> Ranking {
    let retrieval = Rc::new(retrieval);
    let mut hits: Vec<Hit> = scored
        .into_iter()
        .filter(|(_, score)| *score > 0.0)
        .map(|(card, score)| Hit {
            card,
            score,
            provenance: Rc::clone(&retrieval),
        })
        .collect();
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.card.name().cmp(right.card.name()))
    });
    Ranking { hits, retrieval }
}
