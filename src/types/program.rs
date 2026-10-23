//! The lattice of the program at hand: the built-in declarations, and the
//! types the program declares on top of them.
//!
//! A `type` statement adds a constructor to the lattice of the program that
//! wrote it, so that `Point(x = 1, y = 2)` has a type the checker can name,
//! `p.x` reads a declared field, and a union of records is matched member by
//! member. The lattice is held per thread: checking installs it as it goes,
//! and evaluation installs the one checking produced, so a value's type is
//! read against the declarations it was built under.

use super::{TypeConstructor, TypeDefinition, TypeError, TypeRef, TypeSystem};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

/// One declared field of a record: its type written over the record's
/// parameters, each a nullary application of its name.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordField {
    /// The field's name.
    pub name: String,
    /// Its type, with the record's parameters unsubstituted.
    pub ty: TypeRef,
    /// Whether the field is written `name? : Type`.
    pub optional: bool,
    /// What a contract reads where the data holds no entry for the field,
    /// written `name: Type = literal` (HQL-0032).
    pub default: Option<crate::data::Data>,
}

/// What a record declares: its parameters and its fields, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordShape {
    /// The record's type parameters, in declaration order.
    pub parameters: Vec<&'static str>,
    /// Its parents, written over the same parameters.
    pub parents: Vec<TypeRef>,
    /// Its fields, in declaration order.
    pub fields: Vec<RecordField>,
}

impl RecordShape {
    /// The type of `field` on an application of the record to `args`, read
    /// from this record or inherited from a parent, with parameters bound.
    #[must_use]
    pub fn field_type(&self, args: &[TypeRef], field: &str) -> Option<TypeRef> {
        if let Some(declared) = self.fields.iter().find(|held| held.name == field) {
            let found = self.bound(&declared.ty, args);
            return Some(if declared.optional {
                TypeRef::optional(found)
            } else {
                found
            });
        }
        self.parents.iter().find_map(|parent| {
            let parent = self.bound(parent, args);
            record(parent.constructor.0)?.field_type(&parent.args, field)
        })
    }

    /// A type written over the record's parameters, with `args` in their
    /// places.
    #[must_use]
    pub fn bound(&self, written: &TypeRef, args: &[TypeRef]) -> TypeRef {
        if written.args.is_empty()
            && let Some(index) = self
                .parameters
                .iter()
                .position(|parameter| *parameter == written.constructor.0)
        {
            return args.get(index).cloned().unwrap_or(TypeRef::NEVER);
        }
        written
            .constructor
            .apply(written.args.iter().map(|arg| self.bound(arg, args)))
    }
}

/// The built-in lattice plus a program's declarations.
#[derive(Debug, Clone)]
pub struct Lattice {
    /// Every constructor the program may name.
    pub system: Arc<TypeSystem>,
    /// The records the program declared, by name.
    pub records: Arc<HashMap<&'static str, RecordShape>>,
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<Lattice>>> = const { RefCell::new(None) };
}

/// The built-in lattice alone, which every program starts from.
fn base() -> Result<Arc<Lattice>, TypeError> {
    static BASE: OnceLock<Result<Arc<Lattice>, TypeError>> = OnceLock::new();
    BASE.get_or_init(|| {
        super::builtin::build().map(|system| {
            Arc::new(Lattice {
                system: Arc::new(system),
                records: Arc::new(HashMap::new()),
            })
        })
    })
    .clone()
}

/// The lattice in force on this thread.
///
/// # Errors
///
/// Returns the built-in lattice's own defect, if it has one.
pub fn current() -> Result<Arc<Lattice>, TypeError> {
    match CURRENT.with(|held| held.borrow().clone()) {
        Some(lattice) => Ok(lattice),
        None => base(),
    }
}

/// Make `lattice` the one in force on this thread.
pub fn install(lattice: Arc<Lattice>) {
    CURRENT.with(|held| *held.borrow_mut() = Some(lattice));
}

/// Start from the built-in lattice again, forgetting a previous program's
/// declarations.
pub fn reset() {
    CURRENT.with(|held| *held.borrow_mut() = None);
}

/// Add a declaration to the lattice in force, with its record shape when it
/// has fields.
///
/// # Errors
///
/// Returns why the declaration cannot join the lattice.
pub fn declare(definition: TypeDefinition, shape: Option<RecordShape>) -> Result<(), TypeError> {
    let held = current()?;
    let mut system = (*held.system).clone();
    system.declare(definition.clone())?;
    let mut records = (*held.records).clone();
    if let Some(shape) = shape {
        records.insert(definition.constructor.0, shape);
    }
    install(Arc::new(Lattice {
        system: Arc::new(system),
        records: Arc::new(records),
    }));
    Ok(())
}

/// Give a type of the lattice in force one more parent: `Orderable`, when
/// the program implements `Ord` for it.
///
/// # Errors
///
/// Returns why the parent cannot be adopted.
pub fn adopt_parent(
    constructor: TypeConstructor,
    parent: TypeConstructor,
) -> Result<(), TypeError> {
    let held = current()?;
    let mut system = (*held.system).clone();
    system.adopt(constructor, parent)?;
    install(Arc::new(Lattice {
        system: Arc::new(system),
        records: Arc::clone(&held.records),
    }));
    Ok(())
}

/// The shape of a record the program declared, or one the standard library
/// declares.
#[must_use]
pub fn record(name: &str) -> Option<RecordShape> {
    if let Some(shape) = current()
        .ok()
        .and_then(|lattice| lattice.records.get(name).cloned())
    {
        return Some(shape);
    }
    crate::library::shape(name)
}

/// The shape a `Data` tree is tested against when a program asks whether it
/// is a `wanted`: a record the program declared, without type parameters.
/// A built-in record, a card kind among them, is established by its source
/// and never read from a tree.
#[must_use]
pub fn tree_shape(wanted: &TypeRef) -> Option<RecordShape> {
    let lattice = current().ok()?;
    let shape = lattice.records.get(wanted.constructor.0)?;
    (wanted.args.is_empty() && shape.parameters.is_empty()).then(|| shape.clone())
}

/// Whether a value of `subject` can be a `wanted`: one is a subtype of the
/// other, or `subject` is `Data` and `wanted` a record a tree is tested
/// against by its shape.
#[must_use]
pub fn may_be(subject: &TypeRef, wanted: &TypeRef) -> bool {
    wanted.is(subject)
        || subject.is(wanted)
        || (*subject == TypeRef::DATA && tree_shape(wanted).is_some())
}

/// Whether a name is a type of the built-in lattice, declared by the binary
/// rather than by a program.
#[must_use]
pub fn is_builtin(name: &str) -> bool {
    base().is_ok_and(|lattice| lattice.system.constructor(name).is_some())
}

/// Whether values of a named type are represented as records: the types a
/// program declares, and the standard library's `Pair` and `Range`. The library's other
/// records, `Hit` or `Card`, have representations of their own and are built
/// by their extensions, never by a program.
#[must_use]
pub fn is_record_valued(name: &str) -> bool {
    if name == "Pair" || name == "Range" {
        return true;
    }
    current()
        .ok()
        .is_some_and(|lattice| lattice.records.contains_key(name))
}

/// The type of `field` on a value of a declared record type.
#[must_use]
pub fn field_type(receiver: &TypeRef, field: &str) -> Option<TypeRef> {
    record(receiver.constructor.0)?.field_type(&receiver.args, field)
}

/// A name a declaration introduces, kept for the life of the process so
/// that a constructor can carry it. One spelling is interned once.
#[must_use]
pub fn intern(name: &str) -> &'static str {
    static NAMES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut names = NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let names = names.get_or_insert_with(HashSet::new);
    if let Some(held) = names.get(name) {
        return held;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(leaked);
    leaked
}

/// The constructor of a name, interned.
#[must_use]
pub fn constructor(name: &str) -> TypeConstructor {
    TypeConstructor(intern(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TypeKind, TypeParameter, Variance};

    #[test]
    fn a_declared_record_joins_the_lattice_and_reads_its_fields() {
        reset();
        declare(
            TypeDefinition {
                constructor: constructor("Holder"),
                kind: TypeKind::Concrete,
                parameters: vec![TypeParameter {
                    name: "T",
                    variance: Variance::Covariant,
                    bound: None,
                }],
                parents: Vec::new(),
                applied: Vec::new(),
            },
            Some(RecordShape {
                parameters: vec!["T"],
                parents: Vec::new(),
                fields: vec![RecordField {
                    name: "held".to_owned(),
                    ty: TypeConstructor("T").apply([]),
                    optional: false,
                    default: None,
                }],
            }),
        )
        .unwrap();
        let holder = constructor("Holder").apply([TypeRef::INT]);
        assert!(holder.is(&holder));
        assert_eq!(field_type(&holder, "held"), Some(TypeRef::INT));
        assert_eq!(field_type(&holder, "other"), None);
        reset();
        assert!(!holder.is(&holder));
    }

    #[test]
    fn interning_keeps_one_spelling() {
        let first = intern("Spelling");
        let second = intern("Spelling");
        assert!(std::ptr::eq(first, second));
    }
}
