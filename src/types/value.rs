//! Runtime values: what an evaluated expression is, what type it has, and
//! the ordering key it carries of its own.
//!
//! How a value prints is [`display`](super::display).

use super::Type;
use crate::data::Data;
use crate::document::{Card, Document, Kind};
use crate::graph::{Edge, Graph};
use crate::search::{Hit, Ranking};
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
