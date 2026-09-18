//! Enforcement: the laws a type is obliged to satisfy, checked rather than
//! described.
//!
//! The [type system card][card] calls `Seq<T>` an *abstract type of law*, and
//! its *Where the laws live* table puts a law in three places: the declaration
//! in [`std/`](../../std/README.md), the law itself in the comment beside it,
//! and enforcement here. It also concedes that today "the third column carries
//! the whole weight, and the first two are prose that a test cannot
//! contradict."
//!
//! These are that third column. Each is a predicate over a marker from
//! [`hyper`](super::hyper), so a law is checked *for a type* rather than
//! asserted about types in general.
//!
//! # The two arms
//!
//! A law is checked at compile time where it can be a Rust obligation, and at
//! run time where it cannot. The split is not a matter of taste:
//!
//! - [`Narrows`] is the compile-time arm. `impl Narrows<Doc> for Card {}` is
//!   an obligation the compiler discharges, and covariance is a generic impl,
//!   so `Seq<ConceptCard> <: Seq<Doc>` follows from `ConceptCard <: Doc`
//!   without anything restating it.
//! - [`narrowing`], [`round_trips`], [`inhabits`] and [`orderable`] are the
//!   run-time arm. They quantify over *values*, which no Rust bound can reach:
//!   whether a hit's score orders it is a fact about the hit.
//!
//! [`narrowing`] straddles the two deliberately. It takes a compile-time
//! witness and checks it against [`Type::is`](super::Type::is), which is the only thing keeping
//! the static account and the match arms from drifting apart.
//!
//! # What does not hold yet
//!
//! `Option<T>` fails [`inhabits`] and [`orderable`], and the reason is a real
//! gap rather than a bug in the law. Presence is unmarked at run time: there
//! is [`Value::Absent`](super::Value::Absent) and no counterpart for presence, so
//! `Option::<Int>::wrap(Some(5))` is [`Value::Int`](super::Value::Int) and reports its type as
//! `Int`. The lattice distinguishes `Option<Int>` from `Int`; the runtime does
//! not. Stating the law is what turns that from an unremarked asymmetry into a
//! question with an owner.
//!
//! [card]: ../../doc/wiki/hql/type-system.hmd

use super::{Carried, HyperType, Narrows};

/// A compile-time narrowing witness agrees with the lattice.
///
/// `Sub: Narrows<Sup>` is discharged by the compiler; this asks whether
/// [`Type::is`](super::Type::is) says the same. The two are written in different places and
/// neither consults the other, so nothing but this holds them together.
#[must_use]
pub fn narrowing<Sub, Sup>() -> bool
where
    Sub: Narrows<Sup>,
    Sup: HyperType,
{
    Sub::lattice().is(&Sup::lattice())
}

/// A carrier survives the trip through [`Value`](super::Value) and back.
///
/// Wrapping loses nothing and reading recovers exactly what was wrapped. A
/// type failing this has a [`Carried::read`] that disagrees with its own
/// [`Carried::wrap`], which is how a hand-written `let Value::Str(q) = … else`
/// goes wrong in an extension.
#[must_use]
pub fn round_trips<T>(carrier: T::Carrier) -> bool
where
    T: Carried,
    T::Carrier: Clone + PartialEq,
{
    T::read(&T::wrap(carrier.clone())) == Some(carrier)
}

/// A wrapped carrier has the type its marker claims.
///
/// Narrowing rather than equality, because a carrier may wrap to something
/// more specific than the type asked for: `Card::wrap` of a concept card is a
/// [`Type::ConceptCard`](super::Type::ConceptCard), which narrows `Card` and is the right answer.
#[must_use]
pub fn inhabits<T: Carried>(carrier: T::Carrier) -> bool {
    T::wrap(carrier).type_of().is(&T::lattice())
}

/// The lattice calls a type orderable exactly when its values have a key.
///
/// This is the law the type system card argues for `Orderable` and that
/// nothing checked: a prefix of an unordered collection is reproducible only
/// because the order comes from the values. A type claiming `<: Orderable`
/// whose values return no [`Value::order_key`](super::Value::order_key) would silently reintroduce
/// discovery order as the tiebreak.
#[must_use]
pub fn orderable<T: Carried>(carrier: T::Carrier) -> bool {
    T::lattice().is_orderable() == T::wrap(carrier).order_key().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hyper::{Card, ConceptCard, Doc, Int, Option, Orderable, Seq, Set, Str};

    #[test]
    fn every_narrowing_witness_agrees_with_the_lattice() {
        assert!(narrowing::<Card, Doc>());
        assert!(narrowing::<ConceptCard, Card>());
        assert!(narrowing::<ConceptCard, Doc>());
        assert!(narrowing::<Card, Orderable>());
        assert!(narrowing::<Int, Orderable>());
    }

    #[test]
    fn covariance_is_derived_rather_than_restated() {
        // Neither of these has an impl of its own: both follow from
        // `ConceptCard: Narrows<Doc>` through the generic impl in `hyper`.
        assert!(narrowing::<Seq<ConceptCard>, Seq<Doc>>());
        assert!(narrowing::<Set<ConceptCard>, Set<Doc>>());
    }

    #[test]
    fn carriers_survive_the_trip_through_value() {
        assert!(round_trips::<Int>(7));
        assert!(round_trips::<Str>("bearer token".into()));
        assert!(round_trips::<Seq<Int>>(vec![1, 2, 3]));
        assert!(round_trips::<Set<Str>>(vec!["a".into(), "b".into()]));
    }

    #[test]
    fn a_wrapped_carrier_has_the_type_it_claims() {
        assert!(inhabits::<Int>(1));
        assert!(inhabits::<Seq<Int>>(vec![1]));
        assert!(inhabits::<Set<Card>>(vec![]));
    }

    #[test]
    fn orderable_agrees_with_the_key_a_value_carries() {
        assert!(orderable::<Int>(1));
        assert!(orderable::<Str>("a".into()));
        // A collection is not orderable, and holds no key of its own.
        assert!(orderable::<Seq<Int>>(vec![1]));
    }

    /// `Option<T>` does not satisfy the laws, and the failure is the finding
    /// rather than the bug: presence is unmarked at run time, so a present
    /// `Option<Int>` is indistinguishable from an `Int`.
    #[test]
    fn presence_is_unmarked_so_option_does_not_inhabit_its_own_type() {
        assert!(!inhabits::<Option<Int>>(Some(5)));
        assert!(inhabits::<Option<Int>>(None));
        assert!(!orderable::<Option<Int>>(Some(5)));
    }
}
