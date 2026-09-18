//! Collection declarations and operation contracts, independent of runtime storage.
//!
//! `Collection<T>` counts occurrences, including duplicates. Its carriers must
//! make `size` the sum of multiplicities and `contains(x)` equivalent to
//! `count(x) > 0`. It promises neither positional order nor collection equality.
//! Sets strengthen multiplicity to at most one; sequences give every occurrence
//! exactly one position in `0..size`. These are obligations on runtime operations,
//! not facts that nominal subtype checking alone can establish.
//!
//! Maps have a separate ancestry: keyed lookup does not make them collections
//! of entries. Their key projections expose either set or sequence semantics.

use super::{
    TypeConstructor, TypeDefinition, TypeError, TypeKind, TypeParameter, TypeRef, TypeSystem,
    Variance,
};

/// The abstract contract of deterministic total ordering.
pub const ORDERABLE: TypeConstructor = TypeConstructor("Orderable");
/// Finite occurrences, with membership, multiplicity, and cardinality.
pub const COLLECTION: TypeConstructor = TypeConstructor("Collection");
/// A collection whose occurrences have stable positions.
pub const SEQ: TypeConstructor = TypeConstructor("Seq");
/// A concrete materialized sequence.
pub const LIST: TypeConstructor = TypeConstructor("List");
/// A concrete collection of distinct values.
pub const SET: TypeConstructor = TypeConstructor("Set");
/// A finite mapping with unique keys and no guaranteed key order.
pub const MAP: TypeConstructor = TypeConstructor("Map");
/// A map whose keys have stable positions.
pub const ORDERED_MAP: TypeConstructor = TypeConstructor("OrderedMap");
/// An ordered map whose key positions follow the keys' intrinsic ordering.
pub const SORTED_MAP: TypeConstructor = TypeConstructor("SortedMap");

/// Where a sorting operation obtains its ordering relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// The element type must satisfy the `Orderable` contract.
    Intrinsic,
    /// The caller has checked an explicit `(T, T) -> Ordering` comparison.
    ///
    /// This supplies an order for one operation without changing `T`'s ancestry.
    ExplicitComparison,
}

/// Build the collection hierarchy; scalar and domain types can be declared afterward.
pub fn type_system() -> Result<TypeSystem, TypeError> {
    let mut system = TypeSystem::new();
    for definition in definitions() {
        system.declare(definition)?;
    }
    Ok(system)
}

/// Collection declarations in ancestor-first order.
pub fn definitions() -> impl Iterator<Item = TypeDefinition> {
    use TypeKind::{Abstract, Concrete};
    [
        (ORDERABLE, Abstract, None, vec![]),
        (COLLECTION, Abstract, None, vec![element()]),
        (SEQ, Abstract, Some(COLLECTION), vec![element()]),
        (LIST, Concrete, Some(SEQ), vec![element()]),
        (SET, Concrete, Some(COLLECTION), vec![element()]),
        (MAP, Concrete, None, map_parameters(None)),
        (ORDERED_MAP, Concrete, Some(MAP), map_parameters(None)),
        (
            SORTED_MAP,
            Concrete,
            Some(ORDERED_MAP),
            map_parameters(Some(ORDERABLE.into())),
        ),
    ]
    .into_iter()
    .map(|(constructor, kind, parent, parameters)| TypeDefinition {
        constructor,
        kind,
        parameters,
        parent,
    })
}

/// The occurrence type accepted by `size`, `count`, and `contains` operations.
///
/// Follows collection ancestry, so a list or a set qualifies and a map does not.
pub fn element_type(system: &TypeSystem, collection: &TypeRef) -> Result<TypeRef, TypeError> {
    let [element] = arguments(system, collection, COLLECTION)?;
    Ok(element)
}

/// The key and value types of a map or any of its subtypes.
pub fn map_types(system: &TypeSystem, map: &TypeRef) -> Result<(TypeRef, TypeRef), TypeError> {
    let [key, value] = arguments(system, map, MAP)?;
    Ok((key, value))
}

/// The key projection of the supplied static map type.
///
/// `Map<K, V>` exposes `Set<K>`; `OrderedMap<K, V>` and its descendants expose
/// `Seq<K>`. This describes a projection, not an inherited field override:
/// `Seq<K>` does not narrow `Set<K>`. A value viewed as an ordinary map must
/// expose set semantics even if its concrete representation retains key order.
pub fn keys_type(system: &TypeSystem, map: &TypeRef) -> Result<TypeRef, TypeError> {
    let (key, _) = map_types(system, map)?;
    let constructor = if system.as_supertype(map, ORDERED_MAP)?.is_some() {
        SEQ
    } else {
        SET
    };
    system.apply(constructor, [key])
}

/// Check a sequence's sorting requirements and return its materialized list type.
///
/// Sets are not sequences. Positional order does not imply that elements are
/// intrinsically orderable; an explicit comparison supplies that operation's order.
pub fn sort_type(
    system: &TypeSystem,
    sequence: &TypeRef,
    order: SortOrder,
) -> Result<TypeRef, TypeError> {
    let [element] = arguments(system, sequence, SEQ)?;
    if order == SortOrder::Intrinsic {
        let expected = ORDERABLE.into();
        if !system.is_subtype(&element, &expected)? {
            return Err(TypeError::NotSubtype {
                actual: element,
                expected,
            });
        }
    }
    system.apply(LIST, [element])
}

fn arguments<const N: usize>(
    system: &TypeSystem,
    reference: &TypeRef,
    constructor: TypeConstructor,
) -> Result<[TypeRef; N], TypeError> {
    let ancestor = system
        .as_supertype(reference, constructor)?
        .ok_or_else(|| TypeError::ExpectedConstructor {
            actual: reference.clone(),
            expected: constructor,
        })?;
    ancestor
        .args
        .try_into()
        .map_err(|args: Vec<TypeRef>| TypeError::Arity {
            constructor,
            expected: N,
            actual: args.len(),
        })
}

fn element() -> TypeParameter {
    TypeParameter {
        name: "T",
        variance: Variance::Covariant,
        bound: None,
    }
}

fn map_parameters(key_bound: Option<TypeRef>) -> Vec<TypeParameter> {
    vec![
        TypeParameter {
            name: "K",
            // Lookup accepts keys, while traversal produces them.
            variance: Variance::Invariant,
            bound: key_bound,
        },
        TypeParameter {
            name: "V",
            variance: Variance::Covariant,
            bound: None,
        },
    ]
}
