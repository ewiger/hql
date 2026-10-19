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
        if let Some(settled) = self.settled() {
            return settled.to_json();
        }
        use serde_json::{Value as J, json};
        match self {
            Self::Traversal(traversal) => J::String(format!("<{}>", traversal.type_of())),
            Self::Vault => J::String("<vault>".to_owned()),
            Self::Unit => J::Null,
            Self::Ordering(order) => J::String(format!("{order:?}")),
            Self::Map(map) => {
                json!({"type": map.type_of().to_string(), "entries": map.entries().iter().map(|(key, value)| json!({"key": key.to_json(), "value": value.to_json()})).collect::<Vec<_>>() })
            }
            Self::Int(value) => J::from(*value),
            Self::Float(value) => serde_json::Number::from_f64(*value).map_or(J::Null, J::Number),
            Self::Bool(value) => J::Bool(*value),
            Self::Str(text) => J::String(text.to_string()),
            Self::Absent(_) => J::Null,
            Self::Present(value) => value.to_json(),
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
            Self::Set(values, _) | Self::List(values, _) => {
                J::Array(values.iter().map(Self::to_json).collect())
            }
            Self::Hit(hit) => json!({
                "value": hit.name(),
                "score": hit.score,
                "retrieval": retrieval_json(&hit.provenance),
            }),
            Self::Ranking(ranking) => json!({
                "retrieval": retrieval_json(&ranking.retrieval),
                "hits": ranking.hits.iter().map(|hit| json!({
                    "value": hit.name(),
                    "title": hit.card().map(|card| card.document.title()),
                    "score": hit.score,
                })).collect::<Vec<_>>(),
            }),
            Self::Function(function) => J::String(format!("<function {}>", function.ty)),
            Self::Formatted(formatted) => J::String(formatted.text.clone()),
            Self::Group(group) => json!({
                "key": group.key.to_json(),
                "items": group.items.iter().map(Self::to_json).collect::<Vec<_>>(),
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
        if let Some(settled) = self.settled() {
            return settled.fmt(f);
        }
        match self {
            Self::Traversal(traversal) => write!(f, "<{}>", traversal.type_of()),
            Self::Vault => f.write_str("<vault>"),
            Self::Unit => f.write_str("()"),
            Self::Ordering(order) => write!(f, "{order:?}"),
            Self::Map(map) => {
                write!(f, "{} {{", map.type_of().constructor)?;
                for (index, (key, value)) in map.entries().iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}: {}", Nested(key), Nested(value))?;
                }
                f.write_str("}")
            }
            Self::Int(value) => write!(f, "{value}"),
            // Debug formatting keeps the decimal point, so 1.0 is not printed as 1.
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Str(text) => write!(f, "{text}"),
            Self::Absent(_) => f.write_str("none"),
            Self::Present(value) => value.fmt(f),
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
            // `{}` is an empty record, so an empty set names its type.
            Self::Set(values, element) if values.is_empty() => write!(f, "Set<{element}>([])"),
            Self::Set(values, _) => write!(f, "{{{}}}", nested(values)),
            Self::List(values, _) => write!(f, "[{}]", nested(values)),
            Self::Hit(hit) => write!(f, "{} ({:.4})", hit.name(), hit.score),
            Self::Ranking(ranking) => {
                let rendered: Vec<String> = ranking
                    .hits
                    .iter()
                    .map(|hit| format!("{} ({:.4})", hit.name(), hit.score))
                    .collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Self::Formatted(formatted) => f.write_str(&formatted.text),
            Self::Function(function) => match &function.name {
                Some(name) => write!(f, "<function {name} : {}>", function.ty),
                None => write!(f, "<function {}>", function.ty),
            },
            Self::Group(group) => {
                write!(f, "{}: [{}]", Nested(&group.key), nested(&group.items))
            }
        }
    }
}

impl Value {
    /// The value as the HQL literal that writes it, a string quoted even when
    /// it stands alone.
    #[must_use]
    pub fn to_literal(&self) -> String {
        Nested(self).to_string()
    }
}

/// A value inside a collection, printed as the literal that writes it: only a
/// value standing alone prints a string bare, so `["a, b"]` stays one element.
struct Nested<'a>(&'a Value);

impl fmt::Display for Nested<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(settled) = self.0.settled() {
            return Nested(&settled).fmt(f);
        }
        match self.0 {
            Value::Str(text) => write!(f, "{text:?}"),
            Value::Present(value) => Nested(value).fmt(f),
            other => other.fmt(f),
        }
    }
}

fn nested(values: &[Value]) -> String {
    values
        .iter()
        .map(|value| Nested(value).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a value as a table, choosing columns by what the value is.
impl Value {
    /// Render the value as a table, choosing columns by what the value is.
    #[must_use]
    pub fn to_table(&self) -> String {
        if let Some(settled) = self.settled() {
            return settled.to_table();
        }
        grids(self)
            .iter()
            .map(Grid::aligned)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render the value as Markdown pipe tables, with the columns
    /// [`to_table`](Self::to_table) chooses.
    #[must_use]
    pub fn to_markdown_table(&self) -> String {
        if let Some(settled) = self.settled() {
            return settled.to_markdown_table();
        }
        grids(self)
            .iter()
            .map(Grid::piped)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Render the value as comma-separated rows, header first, quoted as RFC
    /// 4180 requires.
    #[must_use]
    pub fn to_csv(&self) -> String {
        if let Some(settled) = self.settled() {
            return settled.to_csv();
        }
        let quote = |cell: &str| {
            if cell.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", cell.replace('"', "\"\""))
            } else {
                cell.to_owned()
            }
        };
        grids(self)
            .iter()
            .map(|grid| {
                std::iter::once(
                    grid.headers
                        .iter()
                        .map(|header| quote(header))
                        .collect::<Vec<_>>(),
                )
                .chain(
                    grid.body
                        .iter()
                        .map(|row| row.iter().map(|cell| quote(cell)).collect()),
                )
                .map(|row| row.join(","))
                .collect::<Vec<_>>()
                .join("\r\n")
            })
            .collect::<Vec<_>>()
            .join("\r\n\r\n")
    }

    /// Render the value as Markdown, for a document to carry as its own text.
    ///
    /// A record becomes a table, a sequence a list, and a document or card its
    /// own body. A presentation is written in its document form.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        if let Some(settled) = self.settled() {
            return settled.to_markdown();
        }
        match self {
            Self::Present(value) => value.to_markdown(),
            // Markdown is spliced in as it is; any other format is fenced
            // with its own language, and plain text stays plain.
            Self::Formatted(formatted) => match formatted.format.0 {
                "Markdown" | "Hmd" | "Text" | "String" => formatted.text.clone(),
                other => format!(
                    "```{}\n{}\n```",
                    other.to_lowercase(),
                    formatted.text.trim_end()
                ),
            },
            Self::Doc(doc) => doc.body.trim().to_owned(),
            Self::Card(card) => card.document.body.trim().to_owned(),
            Self::Set(values, _) | Self::List(values, _) => values
                .iter()
                .map(|value| match value.as_card() {
                    Some(card) => format!("- [[{}]]", card.name()),
                    None => format!("- {}", value.to_string().replace('\n', " ")),
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Map(_) | Self::Ranking(_) | Self::Graph(_) => self.to_markdown_table(),
            Self::Data(data) if matches!(data.as_ref(), Data::Map(_)) => self.to_markdown_table(),
            Self::Unit => String::new(),
            other => other.to_string(),
        }
    }
}

/// Rows under named columns, before they are laid out.
struct Grid {
    headers: Vec<String>,
    body: Vec<Vec<String>>,
}

fn grid(headers: &[&str], body: Vec<Vec<String>>) -> Vec<Grid> {
    vec![Grid {
        headers: headers.iter().map(|header| (*header).to_owned()).collect(),
        body,
    }]
}

/// A list of records laid out with one column per key, in the order the keys
/// first appear, and a record's text as written.
fn records(values: &[Value]) -> Option<Vec<Grid>> {
    let maps: Vec<&std::collections::BTreeMap<String, Data>> = values
        .iter()
        .map(|value| match value {
            Value::Data(data) => match data.as_ref() {
                Data::Map(entries) => Some(entries),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<_>>()?;
    if maps.is_empty() {
        return None;
    }
    let mut headers: Vec<String> = Vec::new();
    for map in &maps {
        for key in map.keys() {
            if !headers.contains(key) {
                headers.push(key.clone());
            }
        }
    }
    let cell = |data: Option<&Data>| match data {
        Some(Data::Str(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    let body = maps
        .iter()
        .map(|map| headers.iter().map(|key| cell(map.get(key))).collect())
        .collect();
    Some(vec![Grid { headers, body }])
}

fn grids(value: &Value) -> Vec<Grid> {
    match value {
        Value::Present(value) => grids(value),
        Value::Ranking(ranking) => grid(
            if ranking.element.is_card() {
                &["rank", "score", "card", "title"]
            } else {
                &["rank", "score", "value", "title"]
            },
            ranking
                .hits
                .iter()
                .enumerate()
                .map(|(index, hit)| {
                    vec![
                        (index + 1).to_string(),
                        format!("{:.4}", hit.score),
                        hit.name(),
                        hit.card()
                            .map(|card| card.document.title().to_owned())
                            .unwrap_or_default(),
                    ]
                })
                .collect(),
        ),
        Value::Graph(graph) => {
            let mut nodes = grid(
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
            nodes.extend(grid(
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
            ));
            nodes
        }
        Value::Set(values, _) | Value::List(values, _) => {
            if values.iter().all(|value| value.as_card().is_some()) {
                return grid(
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
            if let Some(records) = records(values) {
                return records;
            }
            grid(
                &["value"],
                values.iter().map(|value| vec![value.to_string()]).collect(),
            )
        }
        Value::Map(map) => grid(
            &["key", "value"],
            map.entries()
                .iter()
                .map(|(key, value)| vec![key.to_string(), value.to_string()])
                .collect(),
        ),
        Value::Data(data) => match data.as_ref() {
            Data::Map(entries) => grid(
                &["key", "value"],
                entries
                    .iter()
                    .map(|(key, value)| vec![key.clone(), value.to_string()])
                    .collect(),
            ),
            other => grid(&["value"], vec![vec![other.to_string()]]),
        },
        other => grid(&["value"], vec![vec![other.to_string()]]),
    }
}

impl Grid {
    fn widths(&self, cell: impl Fn(&str) -> String) -> Vec<usize> {
        let mut widths: Vec<usize> = self
            .headers
            .iter()
            .map(|header| header.chars().count())
            .collect();
        for row in &self.body {
            for (index, text) in row.iter().enumerate() {
                if index < widths.len() {
                    widths[index] = widths[index].max(cell(text).chars().count());
                }
            }
        }
        widths
    }

    /// Columns padded with spaces and a dashed rule, for a terminal.
    fn aligned(&self) -> String {
        let widths = self.widths(str::to_owned);
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
        let head: Vec<String> = self.headers.clone();
        let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
        let mut out = vec![line(&head), line(&rule)];
        out.extend(self.body.iter().map(|row| line(row)));
        out.join("\n")
    }

    /// A Markdown pipe table: a header row, a delimiter row, then the rows.
    /// A pipe in a cell is escaped and a line break becomes a space, so a
    /// cell cannot end its row early.
    fn piped(&self) -> String {
        let escape = |text: &str| text.replace('|', "\\|").replace('\n', " ");
        let widths = self.widths(escape);
        let line = |cells: Vec<String>| {
            let cells: Vec<String> = cells
                .into_iter()
                .enumerate()
                .map(|(index, cell)| {
                    let width = widths.get(index).copied().unwrap_or(0).max(3);
                    format!("{cell:<width$}")
                })
                .collect();
            format!("| {} |", cells.join(" | "))
        };
        let mut out = vec![
            line(self.headers.clone()),
            line(
                widths
                    .iter()
                    .map(|width| "-".repeat((*width).max(3)))
                    .collect(),
            ),
        ];
        out.extend(
            self.body
                .iter()
                .map(|row| line(row.iter().map(|cell| escape(cell)).collect())),
        );
        out.join("\n")
    }
}
