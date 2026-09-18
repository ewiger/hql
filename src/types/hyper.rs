//! The markers: one per type HQL has, and what carries it in Rust.
//!
//! Each marker is zero-sized and shares its name with the HQL type, so
//! `hyper::Card` is HQL's `Card` and [`document::Card`] is the Rust value
//! behind it. Reading `Seq::<Hit<Card>>::lattice()` should feel like reading
//! the HQL type it returns.
//!
//! `Option` is one of these names, so within this module it is the marker and
//! Rust's own is spelled [`Maybe`]. Bending HQL's vocabulary to avoid the
//! collision would be the host language deciding what a type is called, which
//! is the thing the type system card refuses. `Data` and `Presentation` are
//! the same case, and their carriers are aliased for the same reason.

use super::{Carried, HyperType, Narrows, Type, Value};
use crate::data::Data as DataTree;
use crate::document::{self, Document, Kind};
use crate::graph;
use crate::search;
use super::value::Presentation as Rendered;
use core::marker::PhantomData;
use core::option::Option as Maybe;
use std::rc::Rc;

/// Declare a marker whose carrier is one [`Value`] variant, unwrapped.
macro_rules! held {
    ($(#[$doc:meta])* $name:ident, $lattice:expr, $carrier:ty, $variant:path) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl HyperType for $name {
            fn lattice() -> Type {
                $lattice
            }
        }

        impl Carried for $name {
            type Carrier = $carrier;

            fn read(value: &Value) -> Maybe<Self::Carrier> {
                match value {
                    $variant(held) => Maybe::Some(held.clone()),
                    _ => Maybe::None,
                }
            }

            fn wrap(carrier: Self::Carrier) -> Value {
                $variant(carrier)
            }
        }
    };
}

held!(
    /// A signed 64-bit integer.
    Int, Type::Int, i64, Value::Int
);
held!(
    /// A double-precision number.
    Float, Type::Float, f64, Value::Float
);
held!(
    /// A Boolean truth value.
    Bool, Type::Bool, bool, Value::Bool
);
held!(
    /// Text.
    Str, Type::Str, Rc<str>, Value::Str
);
held!(
    /// An open tree whose keys belong to whoever wrote them.
    Data, Type::Data, Rc<DataTree>, Value::Data
);
held!(
    /// A document.
    Doc, Type::Doc, Rc<Document>, Value::Doc
);
held!(
    /// A directed connection.
    Edge, Type::Edge, Rc<graph::Edge>, Value::Edge
);
held!(
    /// A set of nodes and a set of edges over them.
    Graph, Type::Graph, Rc<graph::Graph>, Value::Graph
);
held!(
    /// A renderable result, terminal with respect to semantic processing.
    Presentation,
    Type::Presentation,
    Rc<Rendered>,
    Value::Presentation
);

/// A declaration's result: nothing to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit;

impl HyperType for Unit {
    fn lattice() -> Type {
        Type::Unit
    }
}

impl Carried for Unit {
    type Carrier = ();

    fn read(value: &Value) -> Maybe<()> {
        matches!(value, Value::Unit).then_some(())
    }

    fn wrap((): ()) -> Value {
        Value::Unit
    }
}

/// The base representation type of the knowledge domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card;

/// A card whose subject is a concept, and which occupies a node position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConceptCard;

/// A card whose subject is a relation, and which does not become a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationCard;

/// The card family shares one carrier and separates on [`Kind`], which is why
/// [`Value::type_of`] has to read the card to answer. No static mapping can
/// replace that, and the trait does not try to.
macro_rules! card {
    ($name:ident, $lattice:expr, $kind:pat) => {
        impl HyperType for $name {
            fn lattice() -> Type {
                $lattice
            }
        }

        impl Carried for $name {
            type Carrier = Rc<document::Card>;

            fn read(value: &Value) -> Maybe<Self::Carrier> {
                match value {
                    Value::Card(card) if matches!(card.kind, $kind) => Maybe::Some(Rc::clone(card)),
                    _ => Maybe::None,
                }
            }

            fn wrap(carrier: Self::Carrier) -> Value {
                Value::Card(carrier)
            }
        }
    };
}

card!(Card, Type::Card, Kind::Concept | Kind::Relation);
card!(ConceptCard, Type::ConceptCard, Kind::Concept);
card!(RelationCard, Type::RelationCard, Kind::Relation);

/// An unordered collection of distinct elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Set<T>(PhantomData<T>);

/// An ordered collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seq<T>(PhantomData<T>);

/// Both collections carry a `Vec` of their element's carrier beside the
/// element [`Type`], which is what makes `Vec<Value>` a `Seq<Card>` rather
/// than a heterogeneous bag.
macro_rules! collection {
    ($name:ident, $lattice:expr, $variant:path, $build:ident) => {
        impl<T: Carried> HyperType for $name<T> {
            fn lattice() -> Type {
                $lattice(Box::new(T::lattice()))
            }
        }

        impl<T: Carried> Carried for $name<T> {
            type Carrier = Vec<T::Carrier>;

            fn read(value: &Value) -> Maybe<Self::Carrier> {
                match value {
                    $variant(values, _) => values.iter().map(T::read).collect(),
                    _ => Maybe::None,
                }
            }

            fn wrap(carrier: Self::Carrier) -> Value {
                Value::$build(carrier.into_iter().map(T::wrap).collect(), T::lattice())
            }
        }

        /// Collections are covariant in their element, which is sound while
        /// every value here is immutable. Stating it as a generic impl is what
        /// makes the compiler derive `Seq<ConceptCard> <: Seq<Doc>` from
        /// `ConceptCard <: Doc` rather than a match arm restating it.
        impl<A, B> Narrows<$name<B>> for $name<A>
        where
            A: Narrows<B> + Carried,
            B: Carried,
        {
        }
    };
}

collection!(Set, Type::Set, Value::Set, set);
collection!(Seq, Type::Seq, Value::Seq, seq);

/// A retrieved value with its score and the retrieval behind both.
///
/// The lattice is parametric and the carrier is not: [`search::Hit`] always
/// holds a card, so `Hit<T>` can only ever be read back as `Hit<Card>`. The
/// discrepancy is real and predates this trait — [`Value::type_of`] answers
/// `Hit<Card>` for every hit — but it was invisible while nothing asked a type
/// what carried it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit<T>(PhantomData<T>);

impl<T: Carried> HyperType for Hit<T> {
    fn lattice() -> Type {
        Type::Hit(Box::new(T::lattice()))
    }
}

impl Carried for Hit<Card> {
    type Carrier = Rc<search::Hit>;

    fn read(value: &Value) -> Maybe<Self::Carrier> {
        match value {
            Value::Hit(hit) => Maybe::Some(Rc::clone(hit)),
            _ => Maybe::None,
        }
    }

    fn wrap(carrier: Self::Carrier) -> Value {
        Value::Hit(carrier)
    }
}

/// An ordered collection of hits, relative to one query.
///
/// Carries the same discrepancy as [`Hit`], for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ranking<T>(PhantomData<T>);

impl<T: Carried> HyperType for Ranking<T> {
    fn lattice() -> Type {
        Type::Ranking(Box::new(T::lattice()))
    }
}

impl Carried for Ranking<Card> {
    type Carrier = Rc<search::Ranking>;

    fn read(value: &Value) -> Maybe<Self::Carrier> {
        match value {
            Value::Ranking(ranking) => Maybe::Some(Rc::clone(ranking)),
            _ => Maybe::None,
        }
    }

    fn wrap(carrier: Self::Carrier) -> Value {
        Value::Ranking(carrier)
    }
}

/// A value that may be absent. Absence is an ordinary sum type; there is no
/// null, so the carrier is Rust's own `Option` and nothing is nullable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Option<T>(PhantomData<T>);

impl<T: Carried> HyperType for Option<T> {
    fn lattice() -> Type {
        Type::Option(Box::new(T::lattice()))
    }
}

impl<T: Carried> Carried for Option<T> {
    type Carrier = Maybe<T::Carrier>;

    fn read(value: &Value) -> Maybe<Self::Carrier> {
        match value {
            Value::Absent(_) => Maybe::Some(Maybe::None),
            other => T::read(other).map(Maybe::Some),
        }
    }

    fn wrap(carrier: Self::Carrier) -> Value {
        match carrier {
            Maybe::Some(held) => T::wrap(held),
            Maybe::None => Value::Absent(T::lattice()),
        }
    }
}

/// Anything carrying an ordering key of its own.
///
/// The one type here that is not [`Carried`]. It has no [`Value`] variant
/// because it is an abstract type of law: a card is orderable by having a
/// name, not by being wrapped in anything. Reaching for `Orderable::read`
/// should fail to compile, and does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orderable;

impl HyperType for Orderable {
    fn lattice() -> Type {
        Type::Orderable
    }
}

// The card family, as the checker relates it. `Card <: Doc` and the two kinds
// narrow both, which `Type::is` states as match arms and these state to the
// compiler. `laws::narrowing` holds the two accounts against each other.
impl Narrows<Doc> for Card {}
impl Narrows<Card> for ConceptCard {}
impl Narrows<Doc> for ConceptCard {}
impl Narrows<Card> for RelationCard {}
impl Narrows<Doc> for RelationCard {}

// What carries an ordering key of its own.
impl Narrows<Orderable> for Int {}
impl Narrows<Orderable> for Float {}
impl Narrows<Orderable> for Str {}
impl Narrows<Orderable> for Doc {}
impl Narrows<Orderable> for Card {}
impl Narrows<Orderable> for ConceptCard {}
impl Narrows<Orderable> for RelationCard {}
impl<T: Carried> Narrows<Orderable> for Hit<T> {}

// A ranking is a sequence of hits that also knows its query.
impl<T: Carried> Narrows<Seq<Hit<T>>> for Ranking<T> where Hit<T>: Carried {}

// Covariance for the two constructors that are not collections.
impl<A, B> Narrows<Hit<B>> for Hit<A>
where
    A: Narrows<B> + Carried,
    B: Carried,
{
}

impl<A, B> Narrows<Option<B>> for Option<A>
where
    A: Narrows<B> + Carried,
    B: Carried,
{
}
