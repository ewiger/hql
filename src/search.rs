//! What a retrieval is: a score, the rule that produced it, and the ordering
//! the two together justify.
//!
//! Nothing here computes a score. An extension supplies the scores and this
//! module assembles them into a ranking, which is the split the knowledge model
//! asks for: an index contributes evidence, and what evidence means is not the
//! index's to decide.

use crate::document::Card;
use crate::types::{TypeRef, Value};
use std::sync::Arc;

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

/// A retrieved value, its score, and the retrieval behind both.
#[derive(Debug, Clone)]
pub struct Hit {
    /// What was retrieved.
    pub value: Value,
    /// The type the retrieval ranked, `T` in `Hit<T>`.
    pub element: TypeRef,
    /// The metric result, relative to one query.
    pub score: f64,
    /// The retained rule.
    pub provenance: Arc<Retrieval>,
}

impl Hit {
    /// The card retrieved, when the value is one.
    #[must_use]
    pub fn card(&self) -> Option<&Arc<Card>> {
        match &self.value {
            Value::Card(card) => Some(card),
            _ => None,
        }
    }

    /// How the hit is named in output: a card by its name, anything else as
    /// it prints.
    #[must_use]
    pub fn name(&self) -> String {
        match &self.value {
            Value::Card(card) => card.name().to_owned(),
            other => other.to_string(),
        }
    }
}

/// An ordered collection of hits, with the query its order is relative to.
#[derive(Debug, Clone)]
pub struct Ranking {
    /// The hits, best first.
    pub hits: Vec<Hit>,
    /// The type ranked, `T` in `Ranking<T>`.
    pub element: TypeRef,
    /// The retained rule, shared by every hit.
    pub retrieval: Arc<Retrieval>,
}

/// Assemble scored values of one type into a ranking.
///
/// A value scoring nothing at all is dropped: zero similarity is not a weak
/// answer, it is the absence of one. Ties break on the value's own name, so a
/// prefix never depends on the order a vault happened to load in.
#[must_use]
pub fn rank(scored: Vec<(Value, f64)>, element: TypeRef, retrieval: Retrieval) -> Ranking {
    let retrieval = Arc::new(retrieval);
    let mut hits: Vec<Hit> = scored
        .into_iter()
        .filter(|(_, score)| *score > 0.0)
        .map(|(value, score)| Hit {
            value,
            element: element.clone(),
            score,
            provenance: Arc::clone(&retrieval),
        })
        .collect();
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.name().cmp(&right.name()))
    });
    Ranking {
        hits,
        element,
        retrieval,
    }
}
