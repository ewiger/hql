//! Runtime values, their types, their ordering, and how they print.

use crate::data::Data;
use crate::document::{Card, Document, Kind};
use crate::graph::{Edge, Graph};
use crate::search::{Hit, Ranking};
use crate::types::Type;
use std::fmt;
use std::rc::Rc;

/// A renderable result. Terminal with respect to semantic processing: the
/// host draws it, and nothing downstream asks it a question about knowledge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    /// Which presenter produced it, such as `table` or `json`.
    pub presenter: String,
    /// The rendered text.
    pub text: String,
}

/// A result of evaluating an HQL expression.
#[derive(Debug, Clone)]
pub enum Value {
    /// A declaration's result: nothing to show.
    Unit,
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// A Boolean truth value.
    Bool(bool),
    /// Text.
    Str(Rc<str>),
    /// Absence, carrying the type of what is not there.
    Absent(Type),
    /// An open tree.
    Data(Rc<Data>),
    /// A document.
    Doc(Rc<Document>),
    /// A card.
    Card(Rc<Card>),
    /// A directed connection.
    Edge(Rc<Edge>),
    /// A graph.
    Graph(Rc<Graph>),
    /// An unordered collection, with the type of its elements.
    Set(Rc<Vec<Value>>, Type),
    /// An ordered collection, with the type of its elements.
    Seq(Rc<Vec<Value>>, Type),
    /// One scored result.
    Hit(Rc<Hit>),
    /// An ordered collection of hits.
    Ranking(Rc<Ranking>),
    /// A rendered result.
    Presentation(Rc<Presentation>),
}

impl Value {
    /// A set of values of a known element type.
    #[must_use]
    pub fn set(values: Vec<Self>, element: Type) -> Self {
        Self::Set(Rc::new(values), element)
    }

    /// A sequence of values of a known element type.
    #[must_use]
    pub fn seq(values: Vec<Self>, element: Type) -> Self {
        Self::Seq(Rc::new(values), element)
    }

    /// The type of the value.
    #[must_use]
    pub fn type_of(&self) -> Type {
        match self {
            Self::Unit => Type::Unit,
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Str(_) => Type::Str,
            Self::Absent(element) => Type::Option(Box::new(element.clone())),
            Self::Data(_) => Type::Data,
            Self::Doc(_) => Type::Doc,
            Self::Card(card) => match card.kind {
                Kind::Concept => Type::ConceptCard,
                Kind::Relation => Type::RelationCard,
            },
            Self::Edge(_) => Type::Edge,
            Self::Graph(_) => Type::Graph,
            Self::Set(_, element) => Type::Set(Box::new(element.clone())),
            Self::Seq(_, element) => Type::Seq(Box::new(element.clone())),
            Self::Hit(_) => Type::Hit(Box::new(Type::Card)),
            Self::Ranking(_) => Type::Ranking(Box::new(Type::Card)),
            Self::Presentation(_) => Type::Presentation,
        }
    }

    /// The elements of any collection, in the order they are held.
    #[must_use]
    pub fn elements(&self) -> Option<Vec<Self>> {
        match self {
            Self::Set(values, _) | Self::Seq(values, _) => Some(values.as_ref().clone()),
            Self::Ranking(ranking) => Some(
                ranking
                    .hits
                    .iter()
                    .map(|hit| Self::Hit(Rc::new(hit.clone())))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// The card a value is, or the card a hit retrieved.
    #[must_use]
    pub fn as_card(&self) -> Option<Rc<Card>> {
        match self {
            Self::Card(card) => Some(Rc::clone(card)),
            Self::Hit(hit) => Some(Rc::clone(&hit.card)),
            _ => None,
        }
    }

    /// The value's own ordering key, or `None` when it has no order.
    ///
    /// This is what makes a prefix of an unordered collection reproducible:
    /// the order is a property of the values, never of how they were found.
    #[must_use]
    pub fn order_key(&self) -> Option<Key> {
        match self {
            Self::Int(value) => Some(Key::number(*value as f64)),
            Self::Float(value) => Some(Key::number(*value)),
            Self::Str(text) => Some(Key::text(text)),
            Self::Doc(doc) => Some(Key::text(&doc.name)),
            Self::Card(card) => Some(Key::text(card.name())),
            // Best first, then the card's own key, so equal scores are stable.
            Self::Hit(hit) => Some(Key {
                number: Some(-hit.score),
                text: hit.card.name().to_owned(),
            }),
            _ => None,
        }
    }

    /// Whether the value counts as true where a condition is expected.
    #[must_use]
    pub fn truth(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

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

/// A value's position in its own ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// The numeric part, compared first when both keys have one.
    pub number: Option<f64>,
    /// The textual part, which breaks a numeric tie.
    pub text: String,
}

impl Key {
    fn number(value: f64) -> Self {
        Self {
            number: Some(value),
            text: String::new(),
        }
    }

    fn text(value: &str) -> Self {
        Self {
            number: None,
            text: value.to_owned(),
        }
    }

    /// Compare two keys, numbers first and text as the tiebreak.
    #[must_use]
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self.number, other.number) {
            (Some(left), Some(right)) => left
                .total_cmp(&right)
                .then_with(|| self.text.cmp(&other.text)),
            _ => self.text.cmp(&other.text),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left == right,
            (Self::Absent(left), Self::Absent(right)) => left == right,
            (Self::Data(left), Self::Data(right)) => left == right,
            // Heavier values compare by what identifies them, not structurally.
            (Self::Doc(left), Self::Doc(right)) => left.path == right.path,
            (Self::Card(left), Self::Card(right)) => left.document.path == right.document.path,
            (Self::Edge(left), Self::Edge(right)) => {
                left.source == right.source && left.target == right.target
            }
            (Self::Set(left, _), Self::Set(right, _))
            | (Self::Seq(left, _), Self::Seq(right, _)) => left == right,
            (Self::Presentation(left), Self::Presentation(right)) => left == right,
            _ => false,
        }
    }
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
