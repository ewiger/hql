//! Semantic search: embeddings, hits and the retrieval that produced them.
//!
//! The embedding is a hashed bag of words compared by cosine. It is offline,
//! deterministic and exact — which is what makes it honest about the one thing
//! that matters here, that `approximate` is `false` and the ranking is
//! reproducible. A trained model would change the numbers, not the shape.

use crate::document::Card;
use std::rc::Rc;

/// The name this implementation reports as the model that produced a vector.
pub const MODEL: &str = "hql.hashbag.v1";
/// How two embeddings are compared.
pub const METRIC: &str = "cosine";

const DIMENSIONS: usize = 256;

/// A vector representation of some text.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding(Vec<f64>);

impl Embedding {
    /// Embed text as an L2-normalized hashed bag of its words.
    #[must_use]
    pub fn of(text: &str) -> Self {
        let mut weights = vec![0.0_f64; DIMENSIONS];
        for word in words(text) {
            weights[bucket(&word)] += 1.0;
        }
        // Damp repetition, so a long document does not outrank a precise one
        // merely by saying the same word more often.
        for weight in &mut weights {
            if *weight > 0.0 {
                *weight = 1.0 + weight.ln();
            }
        }
        let norm = weights.iter().map(|w| w * w).sum::<f64>().sqrt();
        if norm > 0.0 {
            for weight in &mut weights {
                *weight /= norm;
            }
        }
        Self(weights)
    }

    /// Cosine similarity, in `0.0..=1.0` for these non-negative vectors.
    #[must_use]
    pub fn similarity(&self, other: &Self) -> f64 {
        self.0
            .iter()
            .zip(&other.0)
            .map(|(left, right)| left * right)
            .sum()
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() > 1)
        .map(str::to_lowercase)
        .collect()
}

/// FNV-1a, so the bucket a word lands in never depends on the build.
fn bucket(word: &str) -> usize {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in word.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash % DIMENSIONS as u64) as usize
}

/// The rule a ranking was produced by, retained so a score stays evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retrieval {
    /// What was asked.
    pub query: String,
    /// Which corpus answered.
    pub index: String,
    /// Which model produced the vectors.
    pub model: String,
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

/// Rank a corpus against a query.
///
/// Cards scoring nothing at all are dropped: a zero-similarity card is not a
/// weak answer, it is the absence of one.
#[must_use]
pub fn rank(cards: &[Rc<Card>], query: &str, index: &str) -> Ranking {
    let retrieval = Rc::new(Retrieval {
        query: query.to_owned(),
        index: index.to_owned(),
        model: MODEL.to_owned(),
        metric: METRIC.to_owned(),
        approximate: false,
    });
    let asked = Embedding::of(query);
    let mut hits: Vec<Hit> = cards
        .iter()
        .map(|card| Hit {
            score: asked.similarity(&Embedding::of(&card.document.text())),
            card: Rc::clone(card),
            provenance: Rc::clone(&retrieval),
        })
        .filter(|hit| hit.score > 0.0)
        .collect();
    // Score descending, then the card's own ordering key, so equal scores do
    // not make a prefix depend on the order the vault happened to load in.
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.card.name().cmp(right.card.name()))
    });
    Ranking { hits, retrieval }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_is_one_for_identical_text_and_zero_for_disjoint() {
        let left = Embedding::of("bearer token authorization");
        assert!((left.similarity(&left) - 1.0).abs() < 1e-9);
        let right = Embedding::of("zzzz");
        assert!(left.similarity(&right).abs() < 1e-9);
    }

    #[test]
    fn overlap_scores_between_the_two() {
        let query = Embedding::of("bearer token authorization");
        let partial = Embedding::of("the bearer token is sent in a header");
        let score = query.similarity(&partial);
        assert!(score > 0.0 && score < 1.0, "{score}");
    }

    #[test]
    fn buckets_do_not_depend_on_the_build() {
        assert_eq!(bucket("bearer"), bucket("bearer"));
        assert_ne!(bucket("bearer"), bucket("token"));
    }
}
