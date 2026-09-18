//! Declaration metadata and checked semantic ancestry.

use super::{TypeConstructor, TypeRef};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Whether a declaration describes a contract, a concrete runtime type, or a
/// choice among other types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    /// An uninhabited type, used as the element type of an empty collection.
    Bottom,
    /// A semantic contract with no independent runtime representation.
    Abstract,
    /// A type that requires a runtime representation for its values.
    Concrete,
    /// Exactly the listed alternatives: a value of any member is a value of
    /// the union, and nothing else is. A member may apply the union itself,
    /// as `Seq<Data>` does inside `Data`, which is what makes a tree.
    Union(Vec<TypeRef>),
}

/// How an argument participates in subtyping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variance {
    /// The argument may narrow along with the enclosing type.
    Covariant,
    /// The argument must remain identical.
    Invariant,
}

/// A named argument and its optional upper bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    /// The parameter's name within its declaration.
    pub name: &'static str,
    /// The permitted direction of argument substitution.
    pub variance: Variance,
    /// A closed type that every supplied argument must satisfy.
    pub bound: Option<TypeRef>,
}

/// A declaration whose parents each receive the same arguments, or none when
/// nongeneric.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    /// The constructor introduced by this declaration.
    pub constructor: TypeConstructor,
    /// Whether the declaration has a runtime representation.
    pub kind: TypeKind,
    /// The constructor's parameters in application order.
    pub parameters: Vec<TypeParameter>,
    /// Its immediate semantic ancestors. A supertype set declares several,
    /// and every one of them must hold.
    pub parents: Vec<TypeConstructor>,
}

/// A malformed declaration or type application.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TypeError {
    /// A constructor has no declaration in this system.
    #[error("unknown type constructor {0}")]
    UnknownConstructor(TypeConstructor),
    /// A declaration would replace an existing constructor.
    #[error("type constructor {0} is already declared")]
    DuplicateConstructor(TypeConstructor),
    /// Two parameters in one declaration have the same name.
    #[error("type parameter {parameter} is repeated in {constructor}")]
    DuplicateParameter {
        /// The declaration containing the repeated name.
        constructor: TypeConstructor,
        /// The repeated parameter name.
        parameter: &'static str,
    },
    /// A contract or a union cannot be the concrete type of a runtime value.
    #[error("{0} is abstract and has no independent runtime representation")]
    AbstractType(TypeRef),
    /// A union needs at least one alternative.
    #[error("union {0} lists no members")]
    EmptyUnion(TypeConstructor),
    /// A union that lists itself admits nothing its other members do not.
    #[error("union {0} lists itself as a member")]
    SelfMember(TypeConstructor),
    /// An operation requires ancestry through a particular constructor.
    #[error("{actual} does not descend from {expected}")]
    ExpectedConstructor {
        /// The type supplied to the operation.
        actual: TypeRef,
        /// The required constructor in its ancestry.
        expected: TypeConstructor,
    },
    /// An operation requires a type to satisfy a stronger contract.
    #[error("{actual} is not a subtype of {expected}")]
    NotSubtype {
        /// The supplied type.
        actual: TypeRef,
        /// The required supertype.
        expected: TypeRef,
    },
    /// An application supplies the wrong number of arguments.
    #[error("{constructor} expects {expected} type arguments, got {actual}")]
    Arity {
        /// The applied constructor.
        constructor: TypeConstructor,
        /// Its declared arity.
        expected: usize,
        /// The supplied arity.
        actual: usize,
    },
    /// A child cannot forward its parameters to its parent safely.
    #[error("{constructor} has incompatible parameters with parent {parent}")]
    IncompatibleParent {
        /// The child declaration.
        constructor: TypeConstructor,
        /// The parent declaration.
        parent: TypeConstructor,
    },
    /// An argument violates a parameter bound.
    #[error("{argument} does not satisfy {parameter}: {bound} in {constructor}")]
    Bound {
        /// The applied constructor.
        constructor: TypeConstructor,
        /// The constrained parameter.
        parameter: &'static str,
        /// The supplied argument.
        argument: TypeRef,
        /// The required supertype.
        bound: TypeRef,
    },
}

/// An append-only set of declarations with acyclic semantic ancestry.
///
/// Ancestry is a directed acyclic graph: a declaration may narrow several
/// parents, and an ancestor reachable along two paths is one ancestor.
#[derive(Debug, Clone, Default)]
pub struct TypeSystem {
    definitions: BTreeMap<TypeConstructor, TypeDefinition>,
}

impl TypeSystem {
    /// Start with no declarations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a constructor after its parents and bound types are declared.
    ///
    /// A union's members are the one exception to declaration order: they may
    /// apply the union being declared, so they are validated once it exists.
    pub fn declare(&mut self, definition: TypeDefinition) -> Result<(), TypeError> {
        let constructor = definition.constructor;
        if self.definitions.contains_key(&constructor) {
            return Err(TypeError::DuplicateConstructor(constructor));
        }
        let mut names = BTreeSet::new();
        for parameter in &definition.parameters {
            if !names.insert(parameter.name) {
                return Err(TypeError::DuplicateParameter {
                    constructor,
                    parameter: parameter.name,
                });
            }
            if let Some(bound) = &parameter.bound {
                self.validate(bound)?;
            }
        }
        for &parent in &definition.parents {
            let ancestor = self.definition(parent)?;
            if (!ancestor.parameters.is_empty()
                && definition.parameters.len() != ancestor.parameters.len())
                || definition
                    .parameters
                    .iter()
                    .zip(&ancestor.parameters)
                    .any(|(child, parent)| {
                        parent.variance == Variance::Invariant
                            && child.variance != Variance::Invariant
                    })
            {
                return Err(TypeError::IncompatibleParent {
                    constructor,
                    parent,
                });
            }
        }
        if let TypeKind::Union(members) = &definition.kind {
            if members.is_empty() {
                return Err(TypeError::EmptyUnion(constructor));
            }
            // `Seq<Data>` inside `Data` is recursion through an argument and
            // terminates; a bare `Data` inside `Data` would not.
            if members
                .iter()
                .any(|member| member.constructor == constructor)
            {
                return Err(TypeError::SelfMember(constructor));
            }
        }
        // Requiring ancestors to exist before insertion rules out cycles.
        let members = match &definition.kind {
            TypeKind::Union(members) => members.clone(),
            TypeKind::Bottom | TypeKind::Abstract | TypeKind::Concrete => Vec::new(),
        };
        self.definitions.insert(constructor, definition);
        if let Some(error) = members
            .iter()
            .find_map(|member| self.validate(member).err())
        {
            self.definitions.remove(&constructor);
            return Err(error);
        }
        Ok(())
    }

    /// Look up the declaration identified by a constructor.
    pub fn definition(&self, constructor: TypeConstructor) -> Result<&TypeDefinition, TypeError> {
        self.definitions
            .get(&constructor)
            .ok_or(TypeError::UnknownConstructor(constructor))
    }

    /// Find a registered constructor by its spelling without allocating a name.
    pub fn constructor(&self, name: &str) -> Option<TypeConstructor> {
        self.definitions
            .keys()
            .copied()
            .find(|constructor| constructor.0 == name)
    }

    /// Construct a type application and check its arguments and inherited bounds.
    pub fn apply(
        &self,
        constructor: TypeConstructor,
        args: impl IntoIterator<Item = TypeRef>,
    ) -> Result<TypeRef, TypeError> {
        let reference = constructor.apply(args);
        self.validate(&reference)?;
        Ok(reference)
    }

    /// Check every nested application, including constraints inherited from parents.
    pub fn validate(&self, reference: &TypeRef) -> Result<(), TypeError> {
        let definition = self.definition(reference.constructor)?;
        if reference.args.len() != definition.parameters.len() {
            return Err(TypeError::Arity {
                constructor: reference.constructor,
                expected: definition.parameters.len(),
                actual: reference.args.len(),
            });
        }
        for argument in &reference.args {
            self.validate(argument)?;
        }
        for definition in self.ancestors(reference.constructor) {
            for (argument, parameter) in reference.args.iter().zip(&definition.parameters) {
                let Some(bound) = &parameter.bound else {
                    continue;
                };
                if !self.narrows(argument, bound) {
                    return Err(TypeError::Bound {
                        constructor: definition.constructor,
                        parameter: parameter.name,
                        argument: argument.clone(),
                        bound: bound.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Check that a runtime value can have this concrete outer type.
    ///
    /// Arguments may be abstract: `List<Orderable>` still has a concrete
    /// representation, while `Seq<Int>` describes a contract on representations.
    /// This checks the declaration, not whether a carrier has been implemented.
    pub fn validate_concrete(&self, reference: &TypeRef) -> Result<(), TypeError> {
        self.validate(reference)?;
        match &self.definition(reference.constructor)?.kind {
            TypeKind::Concrete => Ok(()),
            // A union's values are represented as one of its members.
            TypeKind::Abstract | TypeKind::Bottom | TypeKind::Union(_) => {
                Err(TypeError::AbstractType(reference.clone()))
            }
        }
    }

    /// View a valid type through a constructor in its ancestry, including itself.
    ///
    /// For example, `List<Int>` viewed through `Collection` is `Collection<Int>`.
    /// Unrelated constructors return `None`; undeclared constructors are errors.
    pub fn as_supertype(
        &self,
        reference: &TypeRef,
        constructor: TypeConstructor,
    ) -> Result<Option<TypeRef>, TypeError> {
        self.validate(reference)?;
        self.definition(constructor)?;
        Ok(self
            .ancestors(reference.constructor)
            .find(|definition| definition.constructor == constructor)
            .map(|definition| {
                constructor.apply(
                    reference
                        .args
                        .iter()
                        .take(definition.parameters.len())
                        .cloned(),
                )
            }))
    }

    /// Whether a valid source type satisfies a valid target type's contract.
    pub fn is_subtype(&self, source: &TypeRef, target: &TypeRef) -> Result<bool, TypeError> {
        self.validate(source)?;
        self.validate(target)?;
        Ok(self.narrows(source, target))
    }

    fn narrows(&self, source: &TypeRef, target: &TypeRef) -> bool {
        if self
            .definitions
            .get(&source.constructor)
            .is_some_and(|definition| definition.kind == TypeKind::Bottom)
        {
            return true;
        }
        let declared = self
            .ancestors(source.constructor)
            .find(|definition| definition.constructor == target.constructor)
            .is_some_and(|definition| {
                definition.parameters.len() == target.args.len()
                    && source
                        .args
                        .iter()
                        .zip(&target.args)
                        .zip(&definition.parameters)
                        .all(|((source, target), parameter)| match parameter.variance {
                            Variance::Covariant => self.narrows(source, target),
                            Variance::Invariant => source == target,
                        })
            });
        if declared {
            return true;
        }
        // Membership is the other way into a union: `Int` never declared
        // `Data`, and is one because `Scalar` is listed. Each step either
        // descends into a smaller source or moves to a union declared earlier,
        // so this terminates.
        match self
            .definitions
            .get(&target.constructor)
            .map(|definition| &definition.kind)
        {
            Some(TypeKind::Union(members)) => {
                members.iter().any(|member| self.narrows(source, member))
            }
            Some(TypeKind::Bottom | TypeKind::Abstract | TypeKind::Concrete) | None => false,
        }
    }

    /// A declaration and everything it narrows, nearest first, each once.
    fn ancestors(&self, constructor: TypeConstructor) -> impl Iterator<Item = &TypeDefinition> {
        // Declarations are append-only and checked before insertion, so every
        // parent exists and the walk terminates at the roots.
        let mut found: Vec<&TypeDefinition> = Vec::new();
        let mut pending = VecDeque::from([constructor]);
        while let Some(next) = pending.pop_front() {
            if found.iter().any(|seen| seen.constructor == next) {
                continue;
            }
            let Some(definition) = self.definitions.get(&next) else {
                continue;
            };
            found.push(definition);
            pending.extend(definition.parents.iter().copied());
        }
        found.into_iter()
    }
}
