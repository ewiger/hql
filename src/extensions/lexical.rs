//! The `lexical` extension: matching a query against shared spellings.
//!
//! The embedding is a hashed bag of words compared by cosine. It is offline,
//! deterministic and exact, which is what makes it honest about the one thing
//! that matters: `approximate` is `false` and the ranking is reproducible. It
//! models no meaning, and its name says so — a query and a document that share
//! no word score zero however plainly one answers the other.

use super::collections::{elements, wrong};
use super::{CheckCx, EvalCx, Purity, Step, collection, named_or_first};
use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::document::Card;
use crate::search::{self, Retrieval};
use crate::types::hyper::{Card as CardType, Ranking};
use crate::types::{HyperType, Type, Value};
use std::ops::Range;
use std::rc::Rc;

/// The name this implementation reports as the model that produced a vector.
pub const MODEL: &str = "hql.hashbag.v1";
/// How two embeddings are compared.
pub const METRIC: &str = "cosine";

const DIMENSIONS: usize = 256;

/// Every step the `lexical` extension provides.
pub(crate) static STEPS: &[Step] = &[Step {
    name: "lexical",
    signature: "cards | lexical(\"bearer token authorization\")",
    summary: "Rank a corpus by the words it shares with a query.",
    purity: Purity::Pure,
    check: check_lexical,
    eval: eval_lexical,
}];

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

fn check_lexical(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "lexical", &span)?;
    if !element.is(&Type::Doc) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!("`lexical` ranks documents, not {element}"),
        ));
    }
    let argument = named_or_first(arguments, "query").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`lexical` needs a query: `lexical(\"…\")`")
    })?;
    let query = cx.infer(&argument.value)?;
    if query != Type::Str {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("a query is text, not {query}"),
        ));
    }
    Ok(Ranking::<CardType>::lattice())
}

fn eval_lexical(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "query").ok_or_else(|| wrong("lexical", "a query", &span))?;
    let Value::Str(query) = cx.evaluate(&argument.value)? else {
        return Err(wrong("lexical", "a text query", &span));
    };
    let elements = elements(&input, "lexical", &span)?;
    let cards: Vec<Rc<Card>> = elements.iter().filter_map(Value::as_card).collect();
    if cards.len() != elements.len() {
        return Err(wrong("lexical", "a collection of cards", &span));
    }
    let asked = Embedding::of(&query);
    let scored = cards
        .into_iter()
        .map(|card| {
            let score = asked.similarity(&Embedding::of(&card.document.text()));
            (card, score)
        })
        .collect();
    Ok(Value::Ranking(Rc::new(search::rank(
        scored,
        Retrieval {
            query: query.to_string(),
            index: cx.vault().index_name(),
            model: MODEL.to_owned(),
            revision: env!("CARGO_PKG_VERSION").to_owned(),
            metric: METRIC.to_owned(),
            approximate: false,
        },
    ))))
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
