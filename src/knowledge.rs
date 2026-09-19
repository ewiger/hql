//! Assemble knowledge kinds, effective metadata, and graph edges from documents.

use crate::data::Data;
use crate::document::{Card, Document, Kind};
use crate::graph::{Edge, EdgeKind};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Assemble a card's knowledge layer and check the kind it declares.
///
/// An authored `knowledge.type: Relation` is a claim. When its endpoints do
/// not resolve the claim fails, and the card stays a concept card with the
/// failed claim retained rather than discarded.
pub(crate) fn build_card(
    document: Document,
    names: &BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> Card {
    let mut metadata = match document.header.path("metadata") {
        Some(Data::Map(entries)) => Data::Map(entries.clone()),
        _ => Data::map(),
    };

    let declared = metadata
        .path("knowledge.type")
        .and_then(Data::as_str)
        .map(str::to_owned);
    let endpoints = || {
        let source = metadata
            .path("knowledge.relation.source")
            .and_then(Data::as_str)?;
        let target = metadata
            .path("knowledge.relation.target")
            .and_then(Data::as_str)?;
        Some((source.to_owned(), target.to_owned()))
    };

    let kind = if declared.as_deref() == Some("Relation") {
        match endpoints() {
            Some((source, target)) if names.contains(&source) && names.contains(&target) => {
                Kind::Relation
            }
            _ => {
                warnings.push(format!(
                    "{}: declares knowledge.type: Relation but its endpoints do not resolve; \
                     kept as a ConceptCard",
                    document.name
                ));
                metadata.insert_path(
                    "knowledge.unverified",
                    Data::Str("relation endpoints do not resolve".to_owned()),
                );
                Kind::Concept
            }
        }
    } else {
        Kind::Concept
    };

    Card {
        document,
        metadata,
        kind,
    }
}

/// Collect link edges from references and relation edges from relation cards.
pub(crate) fn build_edges(cards: &[Arc<Card>]) -> Vec<Edge> {
    // A document edge is identified by its endpoints: writing the link twice
    // addresses it twice and merges the annotation.
    let mut links: BTreeMap<(String, String), Data> = BTreeMap::new();
    for card in cards {
        for reference in &card.document.references {
            let key = (card.name().to_owned(), reference.target.clone());
            let entry = links.entry(key).or_insert_with(Data::map);
            if let (Data::Map(into), Data::Map(from)) = (&mut *entry, &reference.data) {
                for (key, value) in from {
                    into.insert(key.clone(), value.clone());
                }
            }
        }
    }

    let mut edges: Vec<Edge> = links
        .into_iter()
        .map(|((source, target), data)| Edge {
            source,
            target,
            data,
            kind: EdgeKind::Link,
        })
        .collect();

    // A knowledge edge is identified by itself, so relation cards never merge.
    for card in cards {
        if card.kind == Kind::Relation
            && let Some((source, target)) = card.endpoints()
        {
            let mut data = Data::map();
            data.insert_path("card", Data::Str(card.name().to_owned()));
            edges.push(Edge {
                source,
                target,
                data,
                kind: EdgeKind::Relation,
            });
        }
    }
    edges
}
