//! Named type constructors, their applications, and the collection hierarchy.
//!
//! Constructors identify declarations; references supply their arguments.
//! Abstract declarations describe contracts without claiming a runtime carrier.
//! [`TypeSystem::validate`] checks applications and bounds;
//! [`TypeSystem::validate_concrete`] additionally checks a value's outer type.
//! Neither proves the behavioral laws of a carrier: [`collections`] records
//! those obligations and supplies the corresponding operation type queries.

pub mod builtin;
pub mod collections;
pub mod display;
mod mapping;
mod system;
mod value;

pub use builtin::named;
pub use mapping::{CollectionError, MapKind, MapValue};
pub use system::{TypeDefinition, TypeError, TypeKind, TypeParameter, TypeSystem, Variance};
pub use value::{Key, Presentation, Value};

use std::fmt;

/// The identity of a type declaration, independent of its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeConstructor(pub &'static str);

/// An application of a constructor to zero or more type arguments.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeRef {
    /// The declaration being applied.
    pub constructor: TypeConstructor,
    /// Arguments in declaration order.
    pub args: Vec<TypeRef>,
}

impl TypeConstructor {
    /// Build a reference, leaving validation to the type system.
    pub fn apply(self, args: impl IntoIterator<Item = TypeRef>) -> TypeRef {
        TypeRef {
            constructor: self,
            args: args.into_iter().collect(),
        }
    }
}

impl From<TypeConstructor> for TypeRef {
    fn from(constructor: TypeConstructor) -> Self {
        constructor.apply([])
    }
}

impl fmt::Display for TypeConstructor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.constructor.fmt(f)?;
        if !self.args.is_empty() {
            f.write_str("<")?;
            for (index, arg) in self.args.iter().enumerate() {
                if index != 0 {
                    f.write_str(", ")?;
                }
                arg.fmt(f)?;
            }
            f.write_str(">")?;
        }
        Ok(())
    }
}
