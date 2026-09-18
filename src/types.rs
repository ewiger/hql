//! The type lattice, its narrowing relation, and what may be ordered.

use std::fmt;

/// A type an HQL expression can have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// A declaration's result: nothing to show.
    Unit,
    /// A signed 64-bit integer, provisionally.
    Int,
    /// A double-precision number, provisionally.
    Float,
    /// A Boolean truth value.
    Bool,
    /// Text.
    Str,
    /// An open tree whose keys belong to whoever wrote them.
    Data,
    /// A document.
    Doc,
    /// The base representation type of the knowledge domain.
    Card,
    /// A card whose subject is a concept, and which occupies a node position.
    ConceptCard,
    /// A card whose subject is a relation, and which does not become a node.
    RelationCard,
    /// A directed connection.
    Edge,
    /// A set of nodes and a set of edges over them.
    Graph,
    /// An unordered collection of distinct elements.
    Set(Box<Type>),
    /// An ordered collection.
    Seq(Box<Type>),
    /// A retrieved value with its score and the retrieval behind both.
    Hit(Box<Type>),
    /// An ordered collection of hits, relative to one query.
    Ranking(Box<Type>),
    /// A value that may be absent.
    Option(Box<Type>),
    /// A renderable result; terminal with respect to semantic processing.
    Presentation,
    /// Anything carrying an ordering key of its own.
    ///
    /// This is an ordinary supertype, not a capability and not a trait: the
    /// key is a property of the value — a card's name, a hit's score — rather
    /// than something an extension supplies for it.
    Orderable,
}

impl Type {
    /// Whether `self` may be used where `other` is expected.
    ///
    /// The card family and the collections are covariant, which is sound while
    /// every value here is immutable.
    #[must_use]
    pub fn is(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }
        if *other == Self::Orderable {
            return matches!(
                self,
                Self::Int
                    | Self::Float
                    | Self::Str
                    | Self::Doc
                    | Self::Card
                    | Self::ConceptCard
                    | Self::RelationCard
                    | Self::Hit(_)
            );
        }
        match (self, other) {
            (Self::ConceptCard | Self::RelationCard, Self::Card | Self::Doc) => true,
            (Self::Card, Self::Doc) => true,
            // A ranking is a sequence of hits that also knows its query.
            (Self::Ranking(element), Self::Seq(item)) => {
                matches!(item.as_ref(), Self::Hit(inner) if element.is(inner))
            }
            (Self::Set(left), Self::Set(right))
            | (Self::Seq(left), Self::Seq(right))
            | (Self::Hit(left), Self::Hit(right))
            | (Self::Option(left), Self::Option(right))
            | (Self::Ranking(left), Self::Ranking(right)) => left.is(right),
            _ => false,
        }
    }

    /// Whether the type narrows [`Type::Orderable`].
    ///
    /// This is what lets a prefix of an unordered collection be reproducible:
    /// the order comes from the values, never from the order they were found
    /// in. A card is orderable because it has a name.
    #[must_use]
    pub fn is_orderable(&self) -> bool {
        self.is(&Self::Orderable)
    }

    /// The element type of a collection.
    #[must_use]
    pub fn element(&self) -> Option<Self> {
        match self {
            Self::Set(element) | Self::Seq(element) => Some(element.as_ref().clone()),
            Self::Ranking(element) => Some(Self::Hit(element.clone())),
            _ => None,
        }
    }

    /// Whether the type is any collection.
    #[must_use]
    pub fn is_collection(&self) -> bool {
        matches!(self, Self::Set(_) | Self::Seq(_) | Self::Ranking(_))
    }

    /// Whether the type is a card of any kind.
    #[must_use]
    pub fn is_card(&self) -> bool {
        matches!(self, Self::Card | Self::ConceptCard | Self::RelationCard)
    }

    /// The type a collection of this shape takes when its elements change.
    #[must_use]
    pub fn with_element(&self, element: Self) -> Self {
        match self {
            Self::Set(_) => Self::Set(Box::new(element)),
            // Narrowing a ranking's elements loses the query, so it degrades
            // to the ordered collection it is.
            Self::Seq(_) | Self::Ranking(_) => Self::Seq(Box::new(element)),
            other => other.clone(),
        }
    }

    /// The least type both arguments are, or `Data` when they share nothing.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        if self.is(other) {
            return other.clone();
        }
        if other.is(self) {
            return self.clone();
        }
        if self.is_card() && other.is_card() {
            return Self::Card;
        }
        Self::Data
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => f.write_str("Unit"),
            Self::Int => f.write_str("Int"),
            Self::Float => f.write_str("Float"),
            Self::Bool => f.write_str("Bool"),
            Self::Str => f.write_str("String"),
            Self::Data => f.write_str("Data"),
            Self::Doc => f.write_str("Doc"),
            Self::Card => f.write_str("Card"),
            Self::ConceptCard => f.write_str("ConceptCard"),
            Self::RelationCard => f.write_str("RelationCard"),
            Self::Edge => f.write_str("Edge"),
            Self::Graph => f.write_str("Graph"),
            Self::Presentation => f.write_str("Presentation"),
            Self::Orderable => f.write_str("Orderable"),
            Self::Set(element) => write!(f, "Set[{element}]"),
            Self::Seq(element) => write!(f, "Seq[{element}]"),
            Self::Hit(element) => write!(f, "Hit[{element}]"),
            Self::Ranking(element) => write!(f, "Ranking[{element}]"),
            Self::Option(element) => write!(f, "Option[{element}]"),
        }
    }
}

/// Resolve a written type name, such as `Set[Card]`.
#[must_use]
pub fn named(name: &str, arguments: &[Type]) -> Option<Type> {
    let single = || arguments.first().cloned().unwrap_or(Type::Data);
    Some(match name {
        "Unit" => Type::Unit,
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "String" => Type::Str,
        "Data" => Type::Data,
        "Doc" => Type::Doc,
        "Card" => Type::Card,
        "ConceptCard" => Type::ConceptCard,
        "RelationCard" => Type::RelationCard,
        "Edge" | "Link" | "Relation" => Type::Edge,
        "Graph" | "KnowledgeGraph" | "HmdGraph" => Type::Graph,
        "Presentation" => Type::Presentation,
        "Orderable" => Type::Orderable,
        "Set" => Type::Set(Box::new(single())),
        "Seq" | "List" => Type::Seq(Box::new(single())),
        "Hit" => Type::Hit(Box::new(single())),
        "Ranking" => Type::Ranking(Box::new(single())),
        "Option" => Type::Option(Box::new(single())),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_family_narrows_towards_doc() {
        assert!(Type::ConceptCard.is(&Type::Card));
        assert!(Type::ConceptCard.is(&Type::Doc));
        assert!(Type::RelationCard.is(&Type::Card));
        assert!(!Type::RelationCard.is(&Type::ConceptCard));
        assert!(!Type::Card.is(&Type::ConceptCard));
    }

    #[test]
    fn a_ranking_is_a_sequence_of_hits() {
        let ranking = Type::Ranking(Box::new(Type::Card));
        assert!(ranking.is(&Type::Seq(Box::new(Type::Hit(Box::new(Type::Card))))));
        assert!(!ranking.is(&Type::Seq(Box::new(Type::Card))));
    }

    #[test]
    fn collections_are_covariant_in_their_element() {
        let concepts = Type::Set(Box::new(Type::ConceptCard));
        assert!(concepts.is(&Type::Set(Box::new(Type::Doc))));
        assert!(!Type::Set(Box::new(Type::Doc)).is(&concepts));
    }

    #[test]
    fn orderable_is_a_supertype_not_a_capability() {
        // `Card <: Orderable` is ordinary ancestry: a card carries its own
        // key. Nothing registers ordering for it from outside.
        assert!(Type::Card.is(&Type::Orderable));
        assert!(Type::ConceptCard.is(&Type::Orderable));
        assert!(Type::Hit(Box::new(Type::Card)).is(&Type::Orderable));
        assert!(!Type::Set(Box::new(Type::Card)).is(&Type::Orderable));
        assert!(!Type::Data.is(&Type::Orderable));
        assert!(!Type::Orderable.is(&Type::Card));
    }
}
