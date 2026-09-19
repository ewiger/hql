//! Finite maps with unique keys and explicit positional guarantees.

use super::{TypeConstructor, TypeRef, Value, builtin, collections};
use std::sync::Arc;

/// Which ordering contract a map exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapKind {
    /// Unique keys, with no positional guarantee.
    Unordered,
    /// Keys retain construction order.
    Ordered,
    /// Keys follow their intrinsic total ordering.
    Sorted,
}

impl MapKind {
    /// The constructor associated with this contract.
    pub fn constructor(self) -> TypeConstructor {
        match self {
            Self::Unordered => collections::MAP,
            Self::Ordered => collections::ORDERED_MAP,
            Self::Sorted => collections::SORTED_MAP,
        }
    }

    /// Recognize a map constructor.
    pub fn of(constructor: TypeConstructor) -> Option<Self> {
        match constructor {
            collections::MAP => Some(Self::Unordered),
            collections::ORDERED_MAP => Some(Self::Ordered),
            collections::SORTED_MAP => Some(Self::Sorted),
            _ => None,
        }
    }
}

/// A collection cannot be constructed from these values.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CollectionError {
    /// A map's key and value sequences must have the same cardinality.
    #[error("a map needs equally many keys and values, got {keys} and {values}")]
    Length {
        /// Number of keys.
        keys: usize,
        /// Number of values.
        values: usize,
    },
    /// A key occurs more than once under HQL equality.
    #[error("duplicate map key {0}")]
    DuplicateKey(String),
    /// Values must inhabit their declared element types.
    #[error("{actual} does not inhabit {expected}")]
    Element {
        /// The value's type.
        actual: TypeRef,
        /// The required type.
        expected: TypeRef,
    },
    /// A sorted map key must provide an intrinsic ordering.
    #[error("map key {0} has no intrinsic ordering")]
    UnorderedKey(String),
    /// A type application violates the declaration registry.
    #[error(transparent)]
    Type(#[from] super::TypeError),
}

/// A materialized map whose entries can only be created through checked construction.
#[derive(Debug, Clone)]
pub struct MapValue {
    entries: Arc<Vec<(Value, Value)>>,
    concrete: MapKind,
    view: MapKind,
    key: TypeRef,
    value: TypeRef,
    value_view: TypeRef,
}

impl MapValue {
    /// Construct unique associations, sorting only when the contract requires it.
    pub fn new(
        kind: MapKind,
        keys: Vec<Value>,
        values: Vec<Value>,
        key: TypeRef,
        value: TypeRef,
    ) -> Result<Self, CollectionError> {
        builtin::system()?.validate(&kind.constructor().apply([key.clone(), value.clone()]))?;
        if keys.len() != values.len() {
            return Err(CollectionError::Length {
                keys: keys.len(),
                values: values.len(),
            });
        }
        let mut entries: Vec<(Value, Value)> = Vec::with_capacity(keys.len());
        for (held_key, held_value) in keys.into_iter().zip(values) {
            for (held, expected) in [(&held_key, &key), (&held_value, &value)] {
                if !held.type_of().is(expected) {
                    return Err(CollectionError::Element {
                        actual: held.type_of(),
                        expected: expected.clone(),
                    });
                }
            }
            if entries.iter().any(|(existing, _)| existing == &held_key) {
                return Err(CollectionError::DuplicateKey(held_key.to_string()));
            }
            entries.push((held_key, held_value));
        }
        if kind == MapKind::Sorted {
            let mut keyed = entries
                .into_iter()
                .map(|entry| {
                    entry
                        .0
                        .order_key()
                        .ok_or_else(|| CollectionError::UnorderedKey(entry.0.to_string()))
                        .map(|order| (order, entry))
                })
                .collect::<Result<Vec<_>, _>>()?;
            keyed.sort_by(|left, right| left.0.compare(&right.0));
            entries = keyed.into_iter().map(|(_, entry)| entry).collect();
        }
        Ok(Self {
            entries: Arc::new(entries),
            concrete: kind,
            view: kind,
            key,
            value_view: value.clone(),
            value,
        })
    }

    /// The map's concrete runtime type.
    pub fn type_of(&self) -> TypeRef {
        self.concrete
            .constructor()
            .apply([self.key.clone(), self.value.clone()])
    }

    /// Borrow the associations for runtime traversal; unordered maps promise no order.
    pub fn entries(&self) -> &[(Value, Value)] {
        &self.entries
    }

    /// Project keys using the map's static view while retaining its concrete storage.
    pub fn keys(&self) -> Value {
        let keys = self.entries.iter().map(|(key, _)| key.clone()).collect();
        match self.view {
            MapKind::Unordered => Value::set(keys, self.key.clone()),
            MapKind::Ordered | MapKind::Sorted => Value::list(keys, self.key.clone()),
        }
    }

    /// Look up a key; missing keys yield an absent value of the mapping's value type.
    pub fn get(&self, key: &Value) -> Value {
        self.entries
            .iter()
            .find(|(held, _)| held == key)
            .map_or_else(
                || Value::Absent(self.value_view.clone()),
                |(_, value)| Value::Present(Arc::new(value.clone().viewed_as(&self.value_view))),
            )
    }

    /// Reborrow the same storage through an already checked supertype.
    pub(crate) fn viewed_as(&self, expected: &TypeRef) -> Self {
        Self {
            view: MapKind::of(expected.constructor).unwrap_or(self.view),
            value_view: expected
                .args
                .get(1)
                .cloned()
                .unwrap_or_else(|| self.value_view.clone()),
            ..self.clone()
        }
    }
}

impl PartialEq for MapValue {
    fn eq(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        match (self.view, other.view) {
            (MapKind::Unordered, MapKind::Unordered) => self
                .entries
                .iter()
                .all(|entry| other.entries.contains(entry)),
            (MapKind::Ordered | MapKind::Sorted, MapKind::Ordered | MapKind::Sorted) => {
                self.entries == other.entries
            }
            _ => false,
        }
    }
}
