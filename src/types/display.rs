//! How a value prints, projects to JSON, and lays out as a table.
//!
//! Rendering is a separate, testable step rather than something only the
//! binary can do, and keeping it out of [`value`](super::value) leaves that
//! module about what a value *is*.

use super::Value;
use crate::data::Data;
use crate::document::Kind;
use std::fmt;

impl Value {
    /// The JSON projection of the value.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::{Value as J, json};
        match self {
            Self::Unit => J::Null,
            Self::Int(value) => J::from(*value),
            Self::Float(value) => serde_json::Number::from_f64(*value).map_or(J::Null, J::Number),
            Self::Bool(value) => J::Bool(*value),
            Self::Str(text) => J::String(text.to_string()),
            Self::Absent(_) => J::Null,
            Self::Data(data) => data.to_json(),
            Self::Doc(doc) => json!({
                "name": doc.name,
                "path": doc.path.display().to_string(),
                "format": doc.format.name(),
                "title": doc.title(),
                "header": doc.header.to_json(),
            }),
            Self::Card(card) => json!({
                "name": card.name(),
                "kind": match card.kind {
                    Kind::Concept => "ConceptCard",
                    Kind::Relation => "RelationCard",
                },
                "title": card.document.title(),
                "path": card.document.path.display().to_string(),
                "header": card.document.header.to_json(),
                "metadata": card.metadata.to_json(),
            }),
            Self::Edge(edge) => json!({
                "source": edge.source,
                "target": edge.target,
                "data": edge.data.to_json(),
            }),
            Self::Graph(graph) => json!({
                "nodes": graph.nodes.iter().map(|name| json!({
                    "name": name,
                    "presence": graph.presence.get(name).map(crate::graph::Presence::label),
                })).collect::<Vec<_>>(),
                "edges": graph.induced().iter().map(|edge| json!({
                    "source": edge.source,
                    "target": edge.target,
                    "data": edge.data.to_json(),
                })).collect::<Vec<_>>(),
            }),
            Self::Set(values, _) | Self::Seq(values, _) => {
                J::Array(values.iter().map(Self::to_json).collect())
            }
            Self::Hit(hit) => json!({
                "card": hit.card.name(),
                "score": hit.score,
                "retrieval": retrieval_json(&hit.provenance),
            }),
            Self::Ranking(ranking) => json!({
                "retrieval": retrieval_json(&ranking.retrieval),
                "hits": ranking.hits.iter().map(|hit| json!({
                    "card": hit.card.name(),
                    "title": hit.card.document.title(),
                    "score": hit.score,
                })).collect::<Vec<_>>(),
            }),
            Self::Presentation(presentation) => json!({
                "presenter": presentation.presenter,
                "text": presentation.text,
            }),
        }
    }
}

fn retrieval_json(retrieval: &crate::search::Retrieval) -> serde_json::Value {
    serde_json::json!({
        "query": retrieval.query,
        "index": retrieval.index,
        "model": retrieval.model,
        "revision": retrieval.revision,
        "metric": retrieval.metric,
        "approximate": retrieval.approximate,
    })
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => f.write_str("()"),
            Self::Int(value) => write!(f, "{value}"),
            // Debug formatting keeps the decimal point, so 1.0 is not printed as 1.
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Str(text) => write!(f, "{text}"),
            Self::Absent(_) => f.write_str("none"),
            Self::Data(data) => write!(f, "{data}"),
            Self::Doc(doc) => write!(f, "{}", doc.name),
            Self::Card(card) => write!(f, "{}", card.name()),
            Self::Edge(edge) => write!(f, "{} -> {}", edge.source, edge.target),
            Self::Graph(graph) => write!(
                f,
                "graph of {} nodes and {} edges",
                graph.nodes.len(),
                graph.induced().len()
            ),
            Self::Set(values, _) | Self::Seq(values, _) => {
                let rendered: Vec<String> = values
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Self::Hit(hit) => write!(f, "{} ({:.4})", hit.card.name(), hit.score),
            Self::Ranking(ranking) => {
                let rendered: Vec<String> = ranking
                    .hits
                    .iter()
                    .map(|hit| format!("{} ({:.4})", hit.card.name(), hit.score))
                    .collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Self::Presentation(presentation) => f.write_str(&presentation.text),
        }
    }
}

/// Render a value as a table, choosing columns by what the value is.
impl Value {
    /// Render the value as a table, choosing columns by what the value is.
    #[must_use]
    pub fn to_table(&self) -> String {
        table(self)
    }
}

fn table(value: &Value) -> String {
    match value {
        Value::Ranking(ranking) => rows(
            &["rank", "score", "card", "title"],
            ranking
                .hits
                .iter()
                .enumerate()
                .map(|(index, hit)| {
                    vec![
                        (index + 1).to_string(),
                        format!("{:.4}", hit.score),
                        hit.card.name().to_owned(),
                        hit.card.document.title().to_owned(),
                    ]
                })
                .collect(),
        ),
        Value::Graph(graph) => {
            let nodes = rows(
                &["node", "presence"],
                graph
                    .nodes
                    .iter()
                    .map(|name| {
                        vec![
                            name.clone(),
                            graph
                                .presence
                                .get(name)
                                .map_or_else(String::new, crate::graph::Presence::label),
                        ]
                    })
                    .collect(),
            );
            let edges = rows(
                &["source", "target", "data"],
                graph
                    .induced()
                    .iter()
                    .map(|edge| {
                        vec![
                            edge.source.clone(),
                            edge.target.clone(),
                            edge.data.to_string(),
                        ]
                    })
                    .collect(),
            );
            format!("{nodes}\n{edges}")
        }
        Value::Set(values, _) | Value::Seq(values, _) => {
            if values.iter().all(|value| value.as_card().is_some()) {
                return rows(
                    &["card", "kind", "title"],
                    values
                        .iter()
                        .filter_map(Value::as_card)
                        .map(|card| {
                            vec![
                                card.name().to_owned(),
                                match card.kind {
                                    Kind::Concept => "ConceptCard".to_owned(),
                                    Kind::Relation => "RelationCard".to_owned(),
                                },
                                card.document.title().to_owned(),
                            ]
                        })
                        .collect(),
                );
            }
            rows(
                &["value"],
                values.iter().map(|value| vec![value.to_string()]).collect(),
            )
        }
        Value::Data(data) => match data.as_ref() {
            Data::Map(entries) => rows(
                &["key", "value"],
                entries
                    .iter()
                    .map(|(key, value)| vec![key.clone(), value.to_string()])
                    .collect(),
            ),
            other => rows(&["value"], vec![vec![other.to_string()]]),
        },
        other => rows(&["value"], vec![vec![other.to_string()]]),
    }
}

fn rows(headers: &[&str], body: Vec<Vec<String>>) -> String {
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|header| header.chars().count())
        .collect();
    for row in &body {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(cell.chars().count());
            }
        }
    }
    let line = |cells: &[String]| {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let width = widths.get(index).copied().unwrap_or(0);
                format!("{cell:<width$}")
            })
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_owned()
    };
    let head: Vec<String> = headers.iter().map(|header| (*header).to_owned()).collect();
    let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut out = vec![line(&head), line(&rule)];
    out.extend(body.iter().map(|row| line(row)));
    out.join("\n")
}
