//! `Data`: the open tree that headers, metadata and edge annotations are.
//!
//! The keys belong to whoever wrote them. Nothing here knows what any of them
//! mean, which is the whole point of the type.

use std::collections::BTreeMap;
use std::fmt;

/// A tree of scalars, lists and string-keyed maps.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    /// An explicitly empty entry.
    Empty,
    /// A truth value.
    Bool(bool),
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// Text.
    Str(String),
    /// An ordered list.
    List(Vec<Data>),
    /// A map, kept sorted so rendering is deterministic.
    Map(BTreeMap<String, Data>),
}

impl Data {
    /// An empty map, the shape a header or metadata layer starts as.
    #[must_use]
    pub fn map() -> Self {
        Self::Map(BTreeMap::new())
    }

    /// Look one key up, or `None` when this is not a map or the key is absent.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(entries) => entries.get(key),
            _ => None,
        }
    }

    /// Follow a dotted path such as `knowledge.type`.
    #[must_use]
    pub fn path(&self, path: &str) -> Option<&Self> {
        path.split('.')
            .try_fold(self, |current, segment| current.get(segment))
    }

    /// Insert into this map, creating intermediate maps along a dotted path.
    pub fn insert_path(&mut self, path: &str, value: Self) {
        let Self::Map(entries) = self else { return };
        match path.split_once('.') {
            None => {
                entries.insert(path.to_owned(), value);
            }
            Some((head, rest)) => {
                entries
                    .entry(head.to_owned())
                    .or_insert_with(Self::map)
                    .insert_path(rest, value);
            }
        }
    }

    /// The text of a string entry, for the places that need a name or a title.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(text) => Some(text),
            _ => None,
        }
    }

    /// The name of the shape, used in diagnostics and in `table` output.
    #[must_use]
    pub fn shape(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Str(_) => "string",
            Self::List(_) => "list",
            Self::Map(_) => "map",
        }
    }

    /// Every word in every string in the tree, for indexing a header.
    pub(crate) fn words(&self, out: &mut Vec<String>) {
        match self {
            Self::Str(text) => out.push(text.clone()),
            Self::List(items) => items.iter().for_each(|item| item.words(out)),
            Self::Map(entries) => entries.values().for_each(|value| value.words(out)),
            _ => {}
        }
    }

    /// The JSON projection of the tree.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Empty => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(*value),
            Self::Int(value) => serde_json::Value::from(*value),
            Self::Float(value) => serde_json::Number::from_f64(*value)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Self::Str(text) => serde_json::Value::String(text.clone()),
            Self::List(items) => {
                serde_json::Value::Array(items.iter().map(Self::to_json).collect())
            }
            Self::Map(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json()))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Str(text) => write!(f, "{text:?}"),
            Self::List(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Map(entries) => {
                f.write_str("{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                f.write_str("}")
            }
        }
    }
}

/// Parse the subset of YAML that document headers use.
///
/// Scalars, nested maps by indentation, block lists and inline `[a, b]` lists.
/// Anything richer is kept as the text it was written as, because a header is
/// an open tree and refusing to load a document over a YAML feature would make
/// the vault less readable, not more correct.
#[must_use]
pub fn parse_header(source: &str) -> Data {
    let lines: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .collect();
    let mut cursor = 0;
    parse_block(&lines, &mut cursor, 0)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn parse_block(lines: &[&str], cursor: &mut usize, indent: usize) -> Data {
    if *cursor < lines.len() && lines[*cursor].trim_start().starts_with("- ") {
        let mut items = Vec::new();
        while *cursor < lines.len() {
            let line = lines[*cursor];
            if indent_of(line) < indent || !line.trim_start().starts_with("- ") {
                break;
            }
            items.push(parse_scalar(
                line.trim_start().trim_start_matches("- ").trim(),
            ));
            *cursor += 1;
        }
        return Data::List(items);
    }

    let mut entries = BTreeMap::new();
    while *cursor < lines.len() {
        let line = lines[*cursor];
        let own = indent_of(line);
        if own < indent {
            break;
        }
        let trimmed = line.trim();
        let Some((key, rest)) = trimmed.split_once(':') else {
            // Not a mapping line at this level; stop rather than guess.
            break;
        };
        let key = key.trim().trim_matches('"').to_owned();
        let rest = rest.trim();
        *cursor += 1;
        if rest.is_empty() {
            let deeper = lines
                .get(*cursor)
                .map_or(own + 1, |next| indent_of(next))
                .max(own + 1);
            let nested = if lines.get(*cursor).is_some_and(|next| indent_of(next) > own) {
                parse_block(lines, cursor, deeper)
            } else {
                Data::Empty
            };
            entries.insert(key, nested);
        } else {
            entries.insert(key, parse_scalar(rest));
        }
    }
    Data::Map(entries)
}

fn parse_scalar(text: &str) -> Data {
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let items = inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(parse_scalar)
            .collect();
        return Data::List(items);
    }
    if (text.starts_with('"') && text.ends_with('"') && text.len() >= 2)
        || (text.starts_with('\'') && text.ends_with('\'') && text.len() >= 2)
    {
        return Data::Str(text[1..text.len() - 1].to_owned());
    }
    match text {
        "true" => return Data::Bool(true),
        "false" => return Data::Bool(false),
        "null" | "~" | "" => return Data::Empty,
        _ => {}
    }
    if let Ok(value) = text.parse::<i64>() {
        return Data::Int(value);
    }
    if let Ok(value) = text.parse::<f64>()
        && value.is_finite()
        && text.contains('.')
    {
        return Data::Float(value);
    }
    Data::Str(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_nesting_and_lists() {
        let header = parse_header(
            "title: Bearer tokens\ntags: [auth, http]\nknowledge:\n  type: Relation\ndraft: false\n",
        );
        assert_eq!(
            header.path("title").and_then(Data::as_str),
            Some("Bearer tokens")
        );
        assert_eq!(
            header.path("knowledge.type").and_then(Data::as_str),
            Some("Relation")
        );
        assert_eq!(header.path("draft"), Some(&Data::Bool(false)));
        assert_eq!(
            header.path("tags"),
            Some(&Data::List(vec![
                Data::Str("auth".to_owned()),
                Data::Str("http".to_owned())
            ]))
        );
    }

    #[test]
    fn parses_block_lists_and_keeps_unknown_text() {
        let header = parse_header("authors:\n  - ada\n  - grace\nnote: 3 o'clock\n");
        assert_eq!(
            header.path("authors"),
            Some(&Data::List(vec![
                Data::Str("ada".to_owned()),
                Data::Str("grace".to_owned())
            ]))
        );
        assert_eq!(
            header.path("note").and_then(Data::as_str),
            Some("3 o'clock")
        );
    }

    #[test]
    fn insert_path_creates_intermediate_maps() {
        let mut data = Data::map();
        data.insert_path("search.model", Data::Str("hql.hashbag.v1".to_owned()));
        assert_eq!(
            data.path("search.model").and_then(Data::as_str),
            Some("hql.hashbag.v1")
        );
    }
}
