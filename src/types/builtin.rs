//! Built-in constructors and the type operations shared by checking and evaluation.

use super::{
    TypeConstructor, TypeDefinition, TypeError, TypeKind, TypeParameter, TypeRef, TypeSystem,
    Variance, collections,
};
use std::sync::OnceLock;

macro_rules! atoms {
    ($($constant:ident => $name:literal),* $(,)?) => {
        impl TypeRef {
            $(#[doc = concat!("The built-in `", $name, "` type.")]
            pub const $constant: Self = Self { constructor: TypeConstructor($name), args: Vec::new() };)*
        }
    };
}

atoms! {
    NEVER => "Never", UNIT => "Unit", INT => "Int", FLOAT => "Float",
    BOOL => "Bool", STR => "String", DATA => "Data", DOC => "Doc", CARD => "Card",
    CONCEPT_CARD => "ConceptCard", RELATION_CARD => "RelationCard", EDGE => "Edge",
    GRAPH => "Graph", PRESENTATION => "Presentation", ORDERABLE => "Orderable",
    ORDERING => "Ordering", SCALAR => "Scalar", DATE => "Date",
}

macro_rules! applications {
    ($($method:ident => $name:literal),* $(,)?) => {
        impl TypeRef {
            $(#[doc = concat!("Apply the `", $name, "` constructor to its element type.")]
            pub fn $method(element: Self) -> Self {
                TypeConstructor($name).apply([element])
            })*
        }
    };
}

applications! { list => "List", seq => "Seq", set => "Set", hit => "Hit", ranking => "Ranking", optional => "Option", collection => "Collection" }

/// The shared built-in declaration registry.
pub fn system() -> Result<&'static TypeSystem, TypeError> {
    static SYSTEM: OnceLock<Result<TypeSystem, TypeError>> = OnceLock::new();
    SYSTEM.get_or_init(build).as_ref().map_err(Clone::clone)
}

fn build() -> Result<TypeSystem, TypeError> {
    let mut system = collections::type_system()?;
    // A leaf of a tree: a value with no parts of its own. `Bool` is one and is
    // not `Orderable`, which is why the two contracts are separate parents.
    let ordered_scalar = [TypeRef::SCALAR, TypeRef::ORDERABLE];
    for (reference, kind, parents) in [
        (TypeRef::NEVER, TypeKind::Bottom, &[][..]),
        (TypeRef::UNIT, TypeKind::Concrete, &[]),
        (TypeRef::SCALAR, TypeKind::Abstract, &[]),
        (TypeRef::BOOL, TypeKind::Concrete, &[TypeRef::SCALAR]),
        (TypeRef::INT, TypeKind::Concrete, &ordered_scalar),
        (TypeRef::FLOAT, TypeKind::Concrete, &ordered_scalar),
        (TypeRef::STR, TypeKind::Concrete, &ordered_scalar),
        // Declared so a schema can name it; no literal produces one yet.
        (TypeRef::DATE, TypeKind::Concrete, &ordered_scalar),
        (
            TypeRef::DATA,
            TypeKind::Union(vec![
                TypeRef::SCALAR,
                TypeRef::seq(TypeRef::DATA),
                collections::MAP.apply([TypeRef::STR, TypeRef::DATA]),
            ]),
            &[],
        ),
        (TypeRef::DOC, TypeKind::Concrete, &[TypeRef::ORDERABLE]),
        (TypeRef::CARD, TypeKind::Concrete, &[TypeRef::DOC]),
        (TypeRef::CONCEPT_CARD, TypeKind::Concrete, &[TypeRef::CARD]),
        (TypeRef::RELATION_CARD, TypeKind::Concrete, &[TypeRef::CARD]),
        (TypeRef::EDGE, TypeKind::Concrete, &[]),
        (TypeRef::GRAPH, TypeKind::Concrete, &[]),
        (TypeRef::PRESENTATION, TypeKind::Concrete, &[]),
        (TypeRef::ORDERING, TypeKind::Concrete, &[]),
    ] {
        system.declare(TypeDefinition {
            kind,
            constructor: reference.constructor,
            parameters: vec![],
            parents: parents.iter().map(|parent| parent.constructor).collect(),
        })?;
    }
    for (name, parent) in [
        ("Option", None),
        ("Hit", Some(collections::ORDERABLE)),
        ("Ranking", None),
    ] {
        system.declare(TypeDefinition {
            constructor: TypeConstructor(name),
            kind: TypeKind::Concrete,
            parameters: vec![TypeParameter {
                name: "T",
                variance: Variance::Covariant,
                bound: None,
            }],
            parents: parent.into_iter().collect(),
        })?;
    }
    Ok(system)
}

/// Resolve a constructor's spelling without instantiating its parameters.
pub fn constructor(name: &str) -> Option<TypeConstructor> {
    let name = match name {
        "Link" | "Relation" => "Edge",
        "KnowledgeGraph" | "HmdGraph" => "Graph",
        other => other,
    };
    system().ok()?.constructor(name)
}

/// Apply a built-in constructor to exactly the arguments supplied.
pub fn named(name: &str, args: &[TypeRef]) -> Option<TypeRef> {
    let constructor = constructor(name)?;
    // Domain graph declarations remain generic vocabulary over monomorphic
    // graph carriers; their collection fields still use the shared registry.
    let args = if matches!(constructor.0, "Graph" | "Edge") {
        &[][..]
    } else {
        args
    };
    system().ok()?.apply(constructor, args.iter().cloned()).ok()
}

impl TypeRef {
    /// Whether this type satisfies the expected type's contract.
    pub fn is(&self, expected: &Self) -> bool {
        if self.constructor.0 == "Ranking" && expected.constructor.0 != "Ranking" {
            return self
                .args
                .first()
                .is_some_and(|element| Self::seq(Self::hit(element.clone())).is(expected));
        }
        system().is_ok_and(|system| system.is_subtype(self, expected).unwrap_or(false))
    }

    /// Whether this type has a runtime representation of its own, rather than
    /// being a contract or a union that other types satisfy.
    pub fn is_concrete(&self) -> bool {
        system().is_ok_and(|system| {
            system
                .definition(self.constructor)
                .is_ok_and(|definition| definition.kind == TypeKind::Concrete)
        })
    }

    /// Whether values of this type carry an intrinsic total ordering.
    pub fn is_orderable(&self) -> bool {
        self.is(&Self::ORDERABLE)
    }

    /// The occurrence type of a collection, including a retrieval's hits.
    pub fn element(&self) -> Option<Self> {
        if self.constructor.0 == "Ranking" {
            return self.args.first().cloned().map(Self::hit);
        }
        collections::element_type(system().ok()?, self).ok()
    }

    /// Whether this is a collection of occurrences.
    pub fn is_collection(&self) -> bool {
        self.element().is_some()
    }

    /// Whether this type belongs to the card family.
    pub fn is_card(&self) -> bool {
        self.is(&Self::CARD)
    }

    /// Whether the collection preserves positions, independently of value order.
    pub fn is_sequence(&self) -> bool {
        self.constructor.0 == "Ranking"
            || system().is_ok_and(|system| {
                system
                    .as_supertype(self, collections::SEQ)
                    .is_ok_and(|value| value.is_some())
            })
    }

    /// Preserve a collection's shape while replacing its element type.
    pub fn with_element(&self, element: Self) -> Self {
        if self.constructor.0 == "Ranking" {
            return Self::list(element);
        }
        if self.is_collection() {
            return self.constructor.apply([element]);
        }
        self.clone()
    }

    /// Remove an optional wrapper when a lifted operation consumes a value.
    pub fn present(&self) -> &Self {
        if self.constructor.0 == "Option" {
            self.args.first().unwrap_or(self)
        } else {
            self
        }
    }

    /// Find a common type, returning none for unrelated value domains.
    pub fn common(&self, other: &Self) -> Option<Self> {
        if self.is(other) {
            return Some(other.clone());
        }
        if other.is(self) {
            return Some(self.clone());
        }
        if self.is_card() && other.is_card() {
            return Some(Self::CARD);
        }
        if self.constructor == other.constructor && self.args.len() == other.args.len() {
            let definition = system().ok()?.definition(self.constructor).ok()?;
            let args = self
                .args
                .iter()
                .zip(&other.args)
                .zip(&definition.parameters)
                .map(|((left, right), parameter)| match parameter.variance {
                    Variance::Covariant => left.common(right),
                    Variance::Invariant => (left == right).then(|| left.clone()),
                })
                .collect::<Option<Vec<_>>>()?;
            return Some(self.constructor.apply(args));
        }
        None
    }
}
