//! The type system: the lattice, the values that inhabit it, and the binding
//! between them.
//!
//! Three things live here, and keeping them in one module is the point.
//!
//! [`Type`] is the **lattice**: what the checker holds when it reads `Set<Card>`
//! out of a program. It is built at run time from parsed text, so it is data
//! rather than a Rust type.
//!
//! [`Value`] is what an evaluated expression **is**: the Rust value carrying
//! the result.
//!
//! [`HyperType`] and [`Carried`] bind the two. Each type HQL has gets a marker
//! implementing the first, which names the lattice type it stands for; a type
//! that values actually inhabit also implements the second, which names the
//! Rust value carrying it and the round trip to [`Value`]. Before them, the
//! two enums were related by [`Value::type_of`] alone — one direction, and no
//! obligation on a lattice case to have a runtime witness at all.
//!
//! # Laws
//!
//! The [type system card][card] calls `Seq<T>` an *abstract type of law*: its
//! content is a law rather than a representation, and `A <: B` asserts that `A`
//! satisfies `B`'s laws. That card's *Where the laws live* table puts
//! enforcement in this crate. [`laws`] is that column, in two arms:
//!
//! - **Compile time.** [`Narrows`] is a witness that one type narrows another.
//!   `impl Narrows<Doc> for Card {}` is checked by the compiler, and covariance
//!   is a generic impl rather than a match arm.
//! - **Run time.** [`laws`] holds the checks that cannot be a Rust bound —
//!   that a carrier round-trips, that it wraps to the type it claims, and that
//!   [`Type::is_orderable`] agrees with [`Value::order_key`].
//!
//! Neither arm reaches HQL's surface. A trait is a host mechanism: HQL programs
//! see typed functions, and `Renderable<T>` stays an extension registration
//! rather than becoming one of these — see the card's *There are no traits*.
//!
//! [card]: ../../doc/wiki/hql/type-system.hmd

pub mod display;
pub mod hyper;
pub mod lattice;
pub mod laws;
pub mod value;

pub use lattice::{Type, named};
pub use value::{Key, Presentation, Value};

/// A type HQL has.
///
/// Implementors are markers: `Card` is a zero-sized stand-in for HQL's `Card`,
/// not the [`document::Card`](crate::document::Card) that carries it. The
/// separation is what lets `Seq<Card>` compose at compile time while
/// [`Type::Seq`] composes at run time.
///
/// Being a `HyperType` says only that the lattice has this type. Whether any
/// value *is* one is [`Carried`]'s question, and the two are separate because
/// one type answers no to it — see there.
pub trait HyperType {
    /// The lattice type this stands for.
    ///
    /// Named for the `lattice` function in `declarations`, which already does
    /// this job for a written annotation.
    fn lattice() -> Type;
}

/// A [`HyperType`] that values inhabit directly, and the Rust value carrying
/// one.
///
/// This is the mapping the two enums previously left implicit: [`Value`] knew
/// its [`Type`] through [`Value::type_of`], and nothing went the other way, so
/// a lattice case with no runtime witness was not an error.
///
/// `Orderable` is the type that implements [`HyperType`] and not this. It has
/// no [`Value`] variant, because it is an abstract type of law and nothing
/// else — a card is orderable by having a name, not by being wrapped in
/// anything. The type system card observes this in prose; splitting the trait
/// is that observation as a compile-time fact, and it is why `Carrier` does
/// not simply live on [`HyperType`].
pub trait Carried: HyperType {
    /// The Rust value a value of this type is made of.
    type Carrier;

    /// Read a runtime value as this type's carrier, when it is one.
    ///
    /// `None` when the value is of some other type. This is the checked half
    /// of the round trip: every hand-written `let Value::Str(q) = … else` in
    /// an extension is one of these, written out longhand.
    fn read(value: &Value) -> core::option::Option<Self::Carrier>;

    /// Build a runtime value from a carrier.
    fn wrap(carrier: Self::Carrier) -> Value;
}

/// A compile-time witness that `Self` narrows `Super`.
///
/// This is the static arm of law checking. Writing `impl Narrows<Doc> for Card
/// {}` obliges the compiler to agree the pair exists; [`laws::narrowing`] then
/// checks the witness against [`Type::is`], so the two cannot drift.
///
/// Covariance is a generic impl rather than a match arm — see [`hyper`]. There
/// is deliberately no reflexive blanket impl: `T: Narrows<T>` would overlap
/// those, and equality is [`Type::is`]'s first arm already.
pub trait Narrows<Super: HyperType>: HyperType {}
