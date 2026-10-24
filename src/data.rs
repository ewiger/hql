//! `Data`: the open tree that headers, metadata and edge annotations are.
//!
//! The keys belong to whoever wrote them. Nothing here knows what any of them
//! mean, which is the whole point of the type.

use crate::decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt;

impl PartialEq for Data {
    /// Structural equality, except that number leaves compare by value: a
    /// decimal, a float and an integer leaf are equal when each reads as the
    /// same `Decimal`, a float by its shortest digits. Comparing all three
    /// through one value keeps the equality transitive.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Decimal(a), Self::Decimal(b)) => a == b,
            (Self::Decimal(a), Self::Float(b)) | (Self::Float(b), Self::Decimal(a)) => {
                Decimal::from_float(*b).is_some_and(|b| *a == b)
            }
            (Self::Decimal(a), Self::Int(b)) | (Self::Int(b), Self::Decimal(a)) => {
                *a == Decimal::from_int(*b)
            }
            (Self::Int(a), Self::Float(b)) | (Self::Float(b), Self::Int(a)) => {
                Decimal::from_float(*b).is_some_and(|b| Decimal::from_int(*a) == b)
            }
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            _ => false,
        }
    }
}

/// A tree of scalars, lists and string-keyed maps.
#[derive(Debug, Clone)]
pub enum Data {
    /// An explicitly empty entry.
    Empty,
    /// A truth value.
    Bool(bool),
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// A number with a decimal point whose digits are kept as written: a
    /// JSON number, or a `Decimal` value. Read as a `Decimal` it is those
    /// digits; read as anything else it is the nearest `Float`.
    Decimal(Decimal),
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
            Self::Decimal(_) => "decimal",
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
            Self::Decimal(value) => decimal_json(value),
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

/// A JSON number with a decimal's digits and scale exactly: `1.50` stays
/// `1.50`.
#[must_use]
pub fn decimal_json(value: &Decimal) -> serde_json::Value {
    value
        .to_string()
        .parse::<serde_json::Number>()
        .map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// Stands in for a number YAML has no scalar of its own for, so the YAML
/// writer passes it through as text to be replaced by its digits. A suffix
/// is added until no key or string of the tree contains it.
const YAML_NUMBER: &str = "hql-yaml-number-6f1d2a";

/// A JSON tree in YAML. Every number is written with the digits it holds,
/// a decimal's scale included, which YAML's own float writing would round.
///
/// # Errors
/// The YAML writer's own failure.
pub fn yaml_text(value: &serde_json::Value) -> Result<String, serde_yaml_ng::Error> {
    let mut marker = YAML_NUMBER.to_owned();
    let mut suffix = 0_u32;
    while mentions(value, &marker) {
        suffix += 1;
        marker = format!("{YAML_NUMBER}{suffix}");
    }
    let text = serde_yaml_ng::to_string(&YamlTree(value, &marker))?;
    if !text.contains(&marker) {
        return Ok(text);
    }
    let pattern = regex::Regex::new(&format!("{marker}_([-+.0-9eE]+)_"))
        .map_err(<serde_yaml_ng::Error as serde::ser::Error>::custom)?;
    Ok(pattern.replace_all(&text, "$1").into_owned())
}

/// Whether a key or a string of the tree contains `marker`.
fn mentions(value: &serde_json::Value, marker: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(marker),
        serde_json::Value::Array(items) => items.iter().any(|item| mentions(item, marker)),
        serde_json::Value::Object(entries) => entries
            .iter()
            .any(|(key, item)| key.contains(marker) || mentions(item, marker)),
        _ => false,
    }
}

/// A JSON tree as YAML serializes it, with its numbers written exactly, each
/// passed through the writer behind the marker.
struct YamlTree<'a>(&'a serde_json::Value, &'a str);

impl serde::Serialize for YamlTree<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        use serde_json::Value as J;
        match self.0 {
            J::Null => serializer.serialize_unit(),
            J::Bool(value) => serializer.serialize_bool(*value),
            J::Number(number) => {
                if let Some(whole) = number.as_i64() {
                    return serializer.serialize_i64(whole);
                }
                if let Some(whole) = number.as_u64() {
                    return serializer.serialize_u64(whole);
                }
                // A Float's digits are the ones YAML writes for it too.
                let text = number.as_str();
                match number.as_f64() {
                    Some(real)
                        if serde_json::Number::from_f64(real)
                            .is_some_and(|written| written.as_str() == text) =>
                    {
                        serializer.serialize_f64(real)
                    }
                    _ => serializer.serialize_str(&format!("{}_{text}_", self.1)),
                }
            }
            J::String(text) => serializer.serialize_str(text),
            J::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(&YamlTree(item, self.1))?;
                }
                sequence.end()
            }
            J::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, item) in entries {
                    map.serialize_entry(key, &YamlTree(item, self.1))?;
                }
                map.end()
            }
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
            Self::Decimal(value) => write!(f, "{value}"),
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
            // The empty map is `{:}`: `{}` is the empty set.
            Self::Map(entries) if entries.is_empty() => f.write_str("{:}"),
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
    fn a_string_like_the_number_marker_is_written_as_text() {
        let tree = serde_json::json!({
            "note": "hql-yaml-number-6f1d2a_1.5_",
            "amount": serde_json::Number::from_string_unchecked("1.50".to_owned()),
        });
        let text = yaml_text(&tree).unwrap();
        assert!(text.contains("amount: 1.50"), "{text}");
        assert!(text.contains("hql-yaml-number-6f1d2a_1.5_"), "{text}");
    }

    #[test]
    fn number_leaves_are_equal_by_value_and_transitively() {
        let (int, decimal, float) = (
            Data::Int(1),
            Data::Decimal(Decimal::parse("1.0").unwrap()),
            Data::Float(1.0),
        );
        assert_eq!(int, decimal);
        assert_eq!(decimal, float);
        assert_eq!(int, float);
        assert_ne!(Data::Int(1), Data::Float(1.5));
        assert_ne!(
            Data::Int(9_007_199_254_740_993),
            Data::Float(9_007_199_254_740_992.0)
        );
    }

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
