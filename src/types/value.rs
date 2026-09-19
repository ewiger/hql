//! Runtime values: what an evaluated expression is, what type it has, and
//! the ordering key it carries of its own.
//!
//! How a value prints is [`display`](super::display).

use super::{MapKind, MapValue, TypeRef};
use crate::data::Data;
use crate::document::{Card, Document, Kind};
use crate::graph::{Edge, Graph};
use crate::search::{Hit, Ranking};
use std::sync::Arc;

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
    /// The result of a total comparison.
    Ordering(std::cmp::Ordering),
    /// A finite map with checked unique keys.
    Map(Arc<MapValue>),
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// A Boolean truth value.
    Bool(bool),
    /// Text.
    Str(Arc<str>),
    /// Absence, carrying the type of what is not there.
    Absent(TypeRef),
    /// Presence retains the optional type instead of erasing its wrapper.
    Present(Arc<Value>),
    /// An open tree.
    Data(Arc<Data>),
    /// A document.
    Doc(Arc<Document>),
    /// A card.
    Card(Arc<Card>),
    /// A directed connection.
    Edge(Arc<Edge>),
    /// A graph.
    Graph(Arc<Graph>),
    /// An unordered collection, with the type of its elements.
    Set(Arc<Vec<Value>>, TypeRef),
    /// An ordered collection, with the type of its elements.
    List(Arc<Vec<Value>>, TypeRef),
    /// One scored result.
    Hit(Arc<Hit>),
    /// An ordered collection of hits.
    Ranking(Arc<Ranking>),
    /// A rendered result.
    Presentation(Arc<Presentation>),
}

impl Value {
    /// Expose a checked annotation's map contract without changing storage.
    pub(crate) fn viewed_as(self, expected: &TypeRef) -> Self {
        match (&self, MapKind::of(expected.constructor)) {
            (Self::Map(map), Some(_)) => Self::Map(Arc::new(map.viewed_as(expected))),
            (Self::Present(value), _) if expected.constructor.0 == "Option" => Self::Present(
                Arc::new(value.as_ref().clone().viewed_as(expected.present())),
            ),
            (Self::List(values, _), _) if expected.is_collection() => {
                let Some(element) = expected.element() else {
                    return self;
                };
                Self::list(
                    values
                        .iter()
                        .cloned()
                        .map(|value| value.viewed_as(&element))
                        .collect(),
                    element,
                )
            }
            (Self::Set(values, _), _) if expected.is_collection() => {
                let Some(element) = expected.element() else {
                    return self;
                };
                Self::set(
                    values
                        .iter()
                        .cloned()
                        .map(|value| value.viewed_as(&element))
                        .collect(),
                    element,
                )
            }
            _ => self,
        }
    }

    /// A set of values of a known element type.
    #[must_use]
    pub fn set(values: Vec<Self>, element: TypeRef) -> Self {
        let mut unique = Vec::with_capacity(values.len());
        for value in values {
            if !unique.contains(&value) {
                unique.push(value);
            }
        }
        Self::Set(Arc::new(unique), element)
    }

    /// A sequence of values of a known element type.
    #[must_use]
    pub fn list(values: Vec<Self>, element: TypeRef) -> Self {
        Self::List(Arc::new(values), element)
    }

    /// The type of the value.
    #[must_use]
    pub fn type_of(&self) -> TypeRef {
        match self {
            Self::Unit => TypeRef::UNIT,
            Self::Ordering(_) => TypeRef::ORDERING,
            Self::Map(map) => map.type_of(),
            Self::Int(_) => TypeRef::INT,
            Self::Float(_) => TypeRef::FLOAT,
            Self::Bool(_) => TypeRef::BOOL,
            Self::Str(_) => TypeRef::STR,
            Self::Absent(element) => TypeRef::optional(element.clone()),
            Self::Present(value) => TypeRef::optional(value.type_of()),
            Self::Data(_) => TypeRef::DATA,
            Self::Doc(_) => TypeRef::DOC,
            Self::Card(card) => match card.kind {
                Kind::Concept => TypeRef::CONCEPT_CARD,
                Kind::Relation => TypeRef::RELATION_CARD,
            },
            Self::Edge(_) => TypeRef::EDGE,
            Self::Graph(_) => TypeRef::GRAPH,
            Self::Set(_, element) => TypeRef::set(element.clone()),
            Self::List(_, element) => TypeRef::list(element.clone()),
            Self::Hit(_) => TypeRef::hit(TypeRef::CARD),
            Self::Ranking(_) => TypeRef::ranking(TypeRef::CARD),
            Self::Presentation(_) => TypeRef::PRESENTATION,
        }
    }

    /// The elements of any collection, in the order they are held.
    #[must_use]
    pub fn elements(&self) -> Option<Vec<Self>> {
        match self {
            Self::Set(values, _) | Self::List(values, _) => Some(values.as_ref().clone()),
            Self::Ranking(ranking) => Some(
                ranking
                    .hits
                    .iter()
                    .map(|hit| Self::Hit(Arc::new(hit.clone())))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// The card a value is, or the card a hit retrieved.
    #[must_use]
    pub fn as_card(&self) -> Option<Arc<Card>> {
        match self {
            Self::Card(card) => Some(Arc::clone(card)),
            Self::Present(value) => value.as_card(),
            Self::Hit(hit) => Some(Arc::clone(&hit.card)),
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
            Self::Int(value) => Some(Key::Integer(*value)),
            Self::Float(value) => Some(Key::Number(if *value == 0.0 { 0.0 } else { *value })),
            Self::Str(text) => Some(Key::text(text)),
            Self::Doc(doc) => Some(Key::text(&doc.name)),
            Self::Card(card) => Some(Key::text(card.name())),
            // Best first, then the card's own key, so equal scores are stable.
            Self::Hit(hit) => Some(Key::Scored(-hit.score, hit.card.name().to_owned())),
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
}

/// A value's intrinsic order, retaining exact integer precision.
#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    /// An exact signed integer.
    Integer(i64),
    /// A finite floating-point number.
    Number(f64),
    /// A text key.
    Text(String),
    /// A retrieval score with a deterministic document tiebreak.
    Scored(f64, String),
}

impl Key {
    fn text(value: &str) -> Self {
        Self::Text(value.to_owned())
    }

    /// Compare compatible keys, with deterministic ordering across key families.
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.cmp(b),
            (Self::Number(a), Self::Number(b)) => a.total_cmp(b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            (Self::Scored(a, x), Self::Scored(b, y)) => a.total_cmp(b).then_with(|| x.cmp(y)),
            _ => self.family().cmp(&other.family()),
        }
    }

    fn family(&self) -> u8 {
        match self {
            Self::Integer(_) => 0,
            Self::Number(_) => 1,
            Self::Text(_) => 2,
            Self::Scored(..) => 3,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::Ordering(a), Self::Ordering(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left == right,
            (Self::Absent(left), Self::Absent(right)) => left == right,
            (Self::Present(left), Self::Present(right)) => left == right,
            (Self::Data(left), Self::Data(right)) => left == right,
            // Heavier values compare by what identifies them, not structurally.
            (Self::Doc(left), Self::Doc(right)) => left.path == right.path,
            (Self::Card(left), Self::Card(right)) => left.document.path == right.document.path,
            (Self::Edge(left), Self::Edge(right)) => {
                left.source == right.source
                    && left.target == right.target
                    && left.data == right.data
                    && left.kind == right.kind
            }
            (Self::Set(left, _), Self::Set(right, _)) => {
                left.len() == right.len() && left.iter().all(|value| right.contains(value))
            }
            (Self::List(left, _), Self::List(right, _)) => left == right,
            (Self::Graph(a), Self::Graph(b)) => Arc::ptr_eq(a, b),
            (Self::Hit(a), Self::Hit(b)) => {
                Arc::ptr_eq(a, b)
                    || (a.card.document.path == b.card.document.path
                        && a.score == b.score
                        && self.to_json() == other.to_json())
            }
            (Self::Ranking(a), Self::Ranking(b)) => {
                Arc::ptr_eq(a, b) || self.to_json() == other.to_json()
            }
            (Self::Presentation(left), Self::Presentation(right)) => left == right,
            _ => false,
        }
    }
}
