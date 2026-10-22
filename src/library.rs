//! The standard library: the declarations in `std/`, compiled into the binary.
//!
//! A built-in record's fields are read from its declaration here, so what a
//! program may read on a `Hit` or a `Card` is what `std/` says and nothing
//! else. The library is checked as one program when it is first needed, the
//! same way a program's own declarations are.

use crate::ast::{Field, Stmt, TypeAnn};
use crate::declarations::Declarations;
use crate::diagnostics::Diagnostic;
use crate::types::{self, TypeRef};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Every module of the standard library, by name, in loading order: the path
/// below `std/` with folders joined by `.`, and a folder's `mod.hql` named for
/// the folder.
///
/// A module may refer only to what an earlier one declares.
pub const MODULES: &[(&str, &str)] = &[
    ("prelude", PRELUDE),
    ("core", include_str!("../std/core.hql")),
    ("collections", include_str!("../std/collections.hql")),
    ("string", include_str!("../std/string.hql")),
    ("seq", include_str!("../std/seq.hql")),
    ("math", include_str!("../std/math.hql")),
    ("option", include_str!("../std/option.hql")),
    ("result", include_str!("../std/result.hql")),
    ("regex", include_str!("../std/regex.hql")),
    ("doc", include_str!("../std/doc.hql")),
    ("graph", include_str!("../std/graph.hql")),
    ("knowledge", include_str!("../std/knowledge.hql")),
    ("retrieval", include_str!("../std/retrieval.hql")),
    ("format", include_str!("../std/format.hql")),
    ("io", include_str!("../std/io/mod.hql")),
    ("io.net", include_str!("../std/io/net.hql")),
    ("io.logging", include_str!("../std/io/logging.hql")),
    ("vault", include_str!("../std/vault.hql")),
];

/// The standard library's implicit import list, `std/prelude.hql`: not a
/// module, but the imports every source unit starts with unless the vault
/// declines them.
pub const PRELUDE: &str = include_str!("../std/prelude.hql");

/// One declaration, kept for reading its fields.
#[derive(Debug)]
struct Record {
    parameters: Vec<String>,
    parents: Vec<TypeAnn>,
    fields: Vec<Field>,
}

/// One external declaration of a standard module: the signature an extension
/// implements, reduced to what binding cross-checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    /// The declared name, which is the operation's name.
    pub name: String,
    /// Whether the declaration takes `self`: a receiver, which `value.f()`
    /// reaches. Without it the first parameter is an ordinary argument.
    pub has_self: bool,
    /// How many parameters are written, `self` excluded.
    pub parameters: usize,
    /// Whether the last parameter collects trailing arguments.
    pub variadic: bool,
    /// The constructor of the declared result type.
    pub result: String,
}

/// Each module written as external declarations, by canonical name, with its
/// declarations in order.
pub type Contracts = Vec<(&'static str, Vec<Contract>)>;

/// The contracts of the standard library: each module written as external
/// declarations, with its declarations in order. A module with none is
/// implemented by the runtime, or documents its operations in comments, and
/// does not appear.
///
/// # Errors
///
/// Returns the first module that does not parse, a defect of the binary.
pub fn contracts() -> Result<Contracts, String> {
    static CONTRACTS: OnceLock<Result<Contracts, String>> = OnceLock::new();
    CONTRACTS
        .get_or_init(|| {
            let mut found = Vec::new();
            for (name, source) in MODULES {
                let program = crate::parser::parse(source)
                    .map_err(|error| format!("std/{name}: {}", error.message()))?;
                let declared: Vec<Contract> = program
                    .statements
                    .iter()
                    .filter_map(|statement| match statement {
                        // A constructor is bound to the binary's
                        // constructors, not to an extension's step.
                        Stmt::External(declaration)
                            if !crate::constructors::is_table_constructor(
                                name,
                                &declaration.name,
                            ) =>
                        {
                            Some(Contract {
                                name: declaration.name.clone(),
                                has_self: declaration.has_self,
                                parameters: declaration.parameters.len(),
                                variadic: declaration
                                    .parameters
                                    .last()
                                    .is_some_and(|parameter| parameter.variadic),
                                result: declaration.result.name.clone(),
                            })
                        }
                        _ => None,
                    })
                    .collect();
                if !declared.is_empty() {
                    found.push((*name, declared));
                }
            }
            Ok(found)
        })
        .clone()
}

/// A bodyless `impl` header of a standard module: the declaration that a
/// type implements a trait natively, which an extension's function bodies
/// satisfy.
#[derive(Debug, Clone)]
pub struct NativeHeader {
    /// The canonical name of the module declaring it.
    pub module: &'static str,
    pub trait_name: String,
    /// The implementing type's name.
    pub target: String,
}

/// Every native implementation the standard library declares, in order.
/// These are the only implementations of a trait for a built-in type: the
/// runtime supplies the bodies, and the headers say which exist.
///
/// # Errors
///
/// Returns the first module that does not parse, a defect of the binary.
pub fn native_impls() -> Result<Vec<NativeHeader>, String> {
    static HEADERS: OnceLock<Result<Vec<NativeHeader>, String>> = OnceLock::new();
    HEADERS
        .get_or_init(|| {
            let mut found = Vec::new();
            for (name, source) in MODULES {
                let program = crate::parser::parse(source)
                    .map_err(|error| format!("std/{name}: {}", error.message()))?;
                found.extend(
                    program
                        .statements
                        .iter()
                        .filter_map(|statement| match statement {
                            Stmt::Impl(declaration) if declaration.native => Some(NativeHeader {
                                module: name,
                                trait_name: declaration.trait_name.clone(),
                                target: declaration.target.name.clone(),
                            }),
                            _ => None,
                        }),
                );
            }
            Ok(found)
        })
        .clone()
}

/// The whole library as one program's source.
#[must_use]
pub fn source() -> String {
    MODULES
        .iter()
        .map(|(_, module)| *module)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check every declaration in the library.
///
/// # Errors
///
/// Returns the first declaration that does not hold, which is a defect of the
/// binary rather than of any program.
pub fn check() -> Result<(), Diagnostic> {
    load().map(drop)
}

/// The bindings the program module starts with over the host's `vault`.
/// They belong to the program module and to no module of the library: a
/// module is a closure and never sees a host binding.
const PROGRAM_BINDINGS: &str = "cards = vault.cards\ngraph = vault.graph\n";

/// The program module's bindings over `vault`, `cards = vault.cards` among
/// them, in the order written. Each is checked and evaluated as its own
/// program over `vault`.
pub(crate) fn bindings() -> Vec<(String, crate::ast::Expr)> {
    let Ok(program) = crate::parser::parse(PROGRAM_BINDINGS) else {
        return Vec::new();
    };
    program
        .statements
        .into_iter()
        .filter_map(|statement| match statement {
            Stmt::Bind { name, value, .. } => Some((name, value)),
            _ => None,
        })
        .collect()
}

fn records() -> Option<&'static HashMap<String, Record>> {
    static RECORDS: OnceLock<Option<HashMap<String, Record>>> = OnceLock::new();
    RECORDS.get_or_init(|| load().ok()).as_ref()
}

fn load() -> Result<HashMap<String, Record>, Diagnostic> {
    let program = crate::parser::parse(&source())?;
    let mut declarations = Declarations::default();
    let mut records = HashMap::new();
    for statement in program.statements {
        let Stmt::Type(declaration) = statement else {
            continue;
        };
        declarations.declare(&declaration)?;
        records.insert(
            declaration.name,
            Record {
                parameters: declaration.parameters,
                parents: declaration.supertypes,
                fields: declaration.fields,
            },
        );
    }
    Ok(records)
}

/// The shape of a record the library declares, with its parameters standing
/// as nullary applications of their names, as
/// [`program`](crate::types::program) reads one.
pub(crate) fn shape(name: &str) -> Option<crate::types::program::RecordShape> {
    use crate::types::program::{RecordField, RecordShape, intern};
    let record = records()?.get(name)?;
    let parameters: Vec<&'static str> = record
        .parameters
        .iter()
        .map(|parameter| intern(parameter))
        .collect();
    let structural = |written: &TypeAnn| structural(written, &parameters);
    Some(RecordShape {
        parents: record.parents.iter().filter_map(structural).collect(),
        fields: record
            .fields
            .iter()
            .map(|field| {
                Some(RecordField {
                    name: field.name.clone(),
                    ty: structural(&field.annotation)?,
                    optional: field.optional,
                })
            })
            .collect::<Option<Vec<_>>>()?,
        parameters,
    })
}

/// A written type as a reference built from names alone: a parameter stays a
/// nullary application of itself, and nothing is validated, since the record's
/// arguments are bound later.
fn structural(written: &TypeAnn, parameters: &[&'static str]) -> Option<TypeRef> {
    if let Some(parameter) = parameters.iter().find(|held| **held == written.name) {
        return Some(crate::types::TypeConstructor(parameter).apply([]));
    }
    let args = written
        .arguments
        .iter()
        .map(|argument| structural(argument, parameters))
        .collect::<Option<Vec<_>>>()?;
    Some(crate::types::builtin::constructor(&written.name)?.apply(args))
}

/// The type of `field` on a value of `receiver`'s type, as the library
/// declares it: on the type itself, or inherited from a parent.
pub(crate) fn field_type(receiver: &TypeRef, field: &str) -> Option<TypeRef> {
    lookup(receiver.constructor.0, &receiver.args, field)
}

fn lookup(name: &str, args: &[TypeRef], field: &str) -> Option<TypeRef> {
    let record = records()?.get(name)?;
    let scope: HashMap<&str, &TypeRef> = record
        .parameters
        .iter()
        .map(String::as_str)
        .zip(args)
        .collect();
    if let Some(declared) = record.fields.iter().find(|held| held.name == field) {
        let found = resolve(&declared.annotation, &scope)?;
        return Some(if declared.optional {
            TypeRef::optional(found)
        } else {
            found
        });
    }
    record.parents.iter().find_map(|parent| {
        let args = parent
            .arguments
            .iter()
            .map(|argument| resolve(argument, &scope))
            .collect::<Option<Vec<_>>>()?;
        lookup(&parent.name, &args, field)
    })
}

/// The lattice type a written field type denotes once the record's
/// parameters are bound.
fn resolve(written: &TypeAnn, scope: &HashMap<&str, &TypeRef>) -> Option<TypeRef> {
    if let Some(bound) = scope.get(written.name.as_str()) {
        return Some((*bound).clone());
    }
    let args = written
        .arguments
        .iter()
        .map(|argument| resolve(argument, scope))
        .collect::<Option<Vec<_>>>()?;
    types::named(&written.name, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_checks() {
        assert_eq!(check(), Ok(()));
        assert!(records().is_some());
    }

    #[test]
    fn every_field_of_a_built_in_record_has_a_built_in_type() {
        // A field whose type the lattice cannot name would read as no field
        // at all, so a declaration the binary owns must use only its types.
        let records = records().expect("the library");
        let mut seen = 0;
        for (name, record) in records {
            if crate::types::builtin::constructor(name).is_none() {
                continue;
            }
            let args = vec![TypeRef::DATA; record.parameters.len()];
            for field in &record.fields {
                assert!(
                    lookup(name, &args, &field.name).is_some(),
                    "`{name}.{}` names a type the binary does not hold",
                    field.name
                );
                seen += 1;
            }
        }
        assert!(seen >= 20, "only {seen} fields were read");
    }

    #[test]
    fn a_field_is_inherited_and_its_parameters_are_bound() {
        let hit = TypeRef::hit(TypeRef::CARD);
        assert_eq!(field_type(&hit, "value"), Some(TypeRef::CARD));
        assert_eq!(field_type(&hit, "score"), Some(TypeRef::FLOAT));
        assert_eq!(
            field_type(&TypeRef::ranking(TypeRef::INT), "hits"),
            Some(TypeRef::list(TypeRef::hit(TypeRef::INT)))
        );
        assert_eq!(
            field_type(&TypeRef::CONCEPT_CARD, "title"),
            Some(TypeRef::STR)
        );
        assert_eq!(field_type(&TypeRef::CARD, "kind"), Some(TypeRef::STR));
        assert_eq!(field_type(&TypeRef::DOC, "kind"), None);
        assert_eq!(field_type(&TypeRef::DOC, "metadata"), None);
    }
}
