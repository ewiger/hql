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
    /// Collect every string in this tree, depth-first in key order.
    pub fn words(&self, out: &mut Vec<String>) {
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

/// A YAML syntax error or a value that cannot be represented faithfully as Data.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The YAML parser rejected the source.
    #[error("invalid YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
    /// Data cannot represent this YAML value.
    #[error("unsupported YAML value: {0}")]
    Unsupported(&'static str),
    /// Document headers and reference annotations must have string keys.
    #[error("a header or reference annotation must be a YAML mapping")]
    MappingRequired,
}

impl TryFrom<serde_yaml_ng::Value> for Data {
    type Error = Error;

    fn try_from(value: serde_yaml_ng::Value) -> Result<Self, Self::Error> {
        use serde_yaml_ng::Value as Y;
        Ok(match value {
            Y::Null => Self::Empty,
            Y::Bool(value) => Self::Bool(value),
            Y::String(value) => Self::Str(value),
            Y::Number(value) => {
                if let Some(number) = value.as_i64() {
                    Self::Int(number)
                } else if value.is_u64() {
                    return Err(Error::Unsupported("integer exceeds signed 64-bit range"));
                } else {
                    let number = value
                        .as_f64()
                        .filter(|number| number.is_finite())
                        .ok_or(Error::Unsupported("numbers must be finite"))?;
                    Self::Float(number)
                }
            }
            Y::Sequence(values) => Self::List(
                values
                    .into_iter()
                    .map(Self::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            Y::Mapping(entries) => {
                let mut data = BTreeMap::new();
                for (key, value) in entries {
                    let Y::String(key) = key else {
                        return Err(Error::Unsupported("map keys must be strings"));
                    };
                    data.insert(key, Self::try_from(value)?);
                }
                Self::Map(data)
            }
            Y::Tagged(_) => {
                return Err(Error::Unsupported(
                    "custom YAML tags have no Data representation",
                ));
            }
        })
    }
}

/// Parse YAML and convert its tree without guessing at unsupported values.
///
/// # Errors
/// Rejects malformed YAML, non-mapping headers, and values outside Data.
pub fn parse_header(source: &str) -> Result<Data, Error> {
    if source
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
    {
        return Ok(Data::map());
    }
    let mut value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source)?;
    value.apply_merge()?;
    let data = Data::try_from(value)?;
    if !matches!(data, Data::Map(_)) {
        return Err(Error::MappingRequired);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_nesting_and_lists() {
        let header = parse_header(
            "title: Bearer tokens\ntags: [auth, http]\nknowledge:\n  type: Relation\ndraft: false\n",
        ).unwrap();
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
        let header = parse_header("authors:\n  - ada\n  - grace\nnote: 3 o'clock\n").unwrap();
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
        data.insert_path("lexical.model", Data::Str("hql.hashbag.v1".to_owned()));
        assert_eq!(
            data.path("lexical.model").and_then(Data::as_str),
            Some("hql.hashbag.v1")
        );
    }
}
