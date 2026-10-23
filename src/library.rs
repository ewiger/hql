//! The standard library: the declarations in `lib/std/`, compiled into the binary.
//!
//! A built-in record's fields are read from its declaration here, so what a
//! program may read on a `Hit` or a `Card` is what `lib/std/` says and nothing
//! else. The library is checked as one program when it is first needed, the
//! same way a program's own declarations are.

use crate::ast::{Field, Stmt, TypeAnn};
use crate::declarations::Declarations;
use crate::diagnostics::Diagnostic;
use crate::types::{self, TypeRef};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Every module of the standard library, by name, in loading order: the
/// folder below `lib/std/` with its segments joined by `.`, each module's
/// declarations in that folder's `mod.hql`.
///
/// Each module is its own import unit: it names, with `import`, every module
/// whose types or traits it writes, and a module imports only modules listed
/// before it, so the order is the import graph's and the graph is acyclic
/// ([`closure`]).
pub const MODULES: &[(&str, &str)] = &[
    ("prelude", PRELUDE),
    ("core", include_str!("../lib/std/core/mod.hql")),
    ("doc", include_str!("../lib/std/doc/mod.hql")),
    ("retrieval", include_str!("../lib/std/retrieval/mod.hql")),
    ("graph", include_str!("../lib/std/graph/mod.hql")),
    ("knowledge", include_str!("../lib/std/knowledge/mod.hql")),
    ("collections", include_str!("../lib/std/collections/mod.hql")),
    ("string", include_str!("../lib/std/string/mod.hql")),
    ("seq", include_str!("../lib/std/seq/mod.hql")),
    ("math", include_str!("../lib/std/math/mod.hql")),
    ("option", include_str!("../lib/std/option/mod.hql")),
    ("result", include_str!("../lib/std/result/mod.hql")),
    ("regex", include_str!("../lib/std/regex/mod.hql")),
    ("format", include_str!("../lib/std/format/mod.hql")),
    ("io", include_str!("../lib/std/io/mod.hql")),
    ("io.net", include_str!("../lib/std/io/net/mod.hql")),
    ("io.logging", include_str!("../lib/std/io/logging/mod.hql")),
    ("vault", include_str!("../lib/std/vault/mod.hql")),
];

/// The modules of the standard library written in HQL rather than declared
/// for an extension: a program imports one by name, `import validators`,
/// and it is loaded as a workspace module is, its bodies run.
pub const WRITTEN: &[(&str, &str)] = &[("validators", include_str!("../lib/std/validators/mod.hql"))];

/// The source of a written module of the standard library.
#[must_use]
pub fn written(name: &str) -> Option<&'static str> {
    WRITTEN
        .iter()
        .find(|(module, _)| *module == name)
        .map(|(_, source)| *source)
}

/// The standard library's implicit import list, `lib/std/prelude.hql`: not a
/// module, but the package's global space, run when the builtin package
/// loads; its imports are what every source unit outside the package starts
/// with unless the vault declines them.
pub const PRELUDE: &str = include_str!("../lib/std/prelude.hql");

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

/// Hold a module graph to the import-unit rules: each module is a closure.
///
/// A type or trait name a module writes, in a declaration's parents, fields,
/// bounds, parameters, result, receiver pattern or `impl` header, resolves
/// to the module's own declaration or to one of a module it imports, and to
/// nothing else. A name no module of the graph declares, `Any`, `Unit` or a
/// function type, is the language's and needs no import. A module imports
/// only modules listed before it, so the listed order is a topological order
/// of the import graph and the graph is acyclic; a module never imports its
/// own child, and the prelude, which is not a module, imports only modules.
///
/// # Errors
///
/// Returns the first module that does not parse or breaks a rule, naming
/// the module, the name and the module declaring it.
pub fn closure(graph: &[(&str, &str)]) -> Result<(), String> {
    let mut parsed = Vec::new();
    for (name, source) in graph {
        let program = crate::parser::parse(source)
            .map_err(|error| format!("std/{name}: {}", error.message()))?;
        parsed.push((*name, program));
    }
    let mut owners: HashMap<&str, &str> = HashMap::new();
    for (module, program) in &parsed {
        for statement in &program.statements {
            let declared = match statement {
                Stmt::Type(declaration) => &declaration.name,
                Stmt::Trait(declaration) => &declaration.name,
                _ => continue,
            };
            if let Some(owner) = owners.insert(declared, module)
                && owner != *module
            {
                return Err(format!(
                    "`{declared}` is declared by both `std/{owner}` and `std/{module}`: a \
                     declaration has one owner"
                ));
            }
        }
    }
    for (position, (module, program)) in parsed.iter().enumerate() {
        let mut visible: Vec<&str> = vec![module];
        for statement in &program.statements {
            let Stmt::Import { path, .. } = statement else {
                continue;
            };
            let imported = path.join(".");
            // The prelude is not a module and runs once the package has
            // loaded, so it may name any module of the graph.
            let reachable = if *module == "prelude" {
                &parsed[..]
            } else {
                &parsed[..position]
            };
            let Some((found, _)) = reachable
                .iter()
                .find(|(held, _)| *held == imported && *held != "prelude")
            else {
                let why = if parsed.iter().any(|(held, _)| *held == imported) {
                    "which is listed after it: a module imports only the modules before it, so \
                     the import graph has no cycle"
                } else {
                    "which is no module of the standard library"
                };
                return Err(format!("`std/{module}` imports `{imported}`, {why}"));
            };
            if imported.starts_with(&format!("{module}.")) {
                return Err(format!(
                    "`std/{module}` imports its own child `{imported}`: what a parent needs from \
                     a child belongs in the parent, or in a module both import"
                ));
            }
            visible.push(found);
        }
        if *module == "prelude" {
            continue;
        }
        let mut written = Vec::new();
        let receiver: Vec<&str> = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Stmt::ModuleFor { pattern, .. } => Some(pattern),
                _ => None,
            })
            .map(|pattern| {
                written.push(pattern);
                pattern
                    .arguments
                    .iter()
                    .map(|argument| argument.name.as_str())
                    .collect()
            })
            .unwrap_or_default();
        for statement in &program.statements {
            let mut variables: Vec<&str> = receiver.clone();
            match statement {
                Stmt::Type(declaration) => {
                    variables.extend(declaration.parameters.iter().map(String::as_str));
                    let mut names = declaration.supertypes.iter().collect::<Vec<_>>();
                    names.extend(declaration.fields.iter().map(|field| &field.annotation));
                    names.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                    names.extend(declaration.union.iter().flatten());
                    check_names(module, &names, &variables, &visible, &owners)?;
                }
                Stmt::Trait(declaration) => {
                    variables.extend(declaration.generics.iter().map(String::as_str));
                    let mut names = declaration.supertraits.iter().collect::<Vec<_>>();
                    names.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                    for method in &declaration.methods {
                        names.extend(
                            method
                                .parameters
                                .iter()
                                .map(|parameter| &parameter.annotation),
                        );
                        names.push(&method.result);
                    }
                    check_names(module, &names, &variables, &visible, &owners)?;
                }
                Stmt::Impl(declaration) => {
                    variables.extend(declaration.generics.iter().map(String::as_str));
                    let header = TypeAnn {
                        name: declaration.trait_name.clone(),
                        arguments: declaration.trait_arguments.clone(),
                        span: declaration.span.clone(),
                    };
                    let mut names = vec![&header, &declaration.target];
                    names.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                    for method in &declaration.methods {
                        names.extend(
                            method
                                .parameters
                                .iter()
                                .map(|parameter| &parameter.annotation),
                        );
                        names.push(&method.result);
                    }
                    check_names(module, &names, &variables, &visible, &owners)?;
                }
                Stmt::External(declaration) => {
                    variables.extend(declaration.generics.iter().map(String::as_str));
                    let mut names = declaration.receiver.iter().collect::<Vec<_>>();
                    names.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                    names.extend(
                        declaration
                            .parameters
                            .iter()
                            .map(|parameter| &parameter.annotation),
                    );
                    names.push(&declaration.result);
                    check_names(module, &names, &variables, &visible, &owners)?;
                }
                Stmt::Fn(declaration) => {
                    variables.extend(declaration.generics.iter().map(String::as_str));
                    let mut names = declaration
                        .bounds
                        .iter()
                        .map(|(_, bound)| bound)
                        .collect::<Vec<_>>();
                    names.extend(
                        declaration
                            .parameters
                            .iter()
                            .map(|parameter| &parameter.annotation),
                    );
                    names.push(&declaration.result);
                    check_names(module, &names, &variables, &visible, &owners)?;
                }
                Stmt::Bind {
                    annotation: Some(annotation),
                    ..
                } => check_names(module, &[annotation], &variables, &visible, &owners)?,
                _ => {}
            }
        }
        check_names(module, &written, &[], &visible, &owners)?;
    }
    Ok(())
}

/// Each name the written types mention, and their arguments', is a variable
/// in scope, visible in the module, or declared by no module of the graph.
fn check_names(
    module: &str,
    written: &[&TypeAnn],
    variables: &[&str],
    visible: &[&str],
    owners: &HashMap<&str, &str>,
) -> Result<(), String> {
    for annotation in written {
        let name = annotation.name.as_str();
        if !variables.contains(&name)
            && let Some(owner) = owners.get(name)
            && !visible.contains(owner)
        {
            return Err(format!(
                "`std/{module}` writes `{name}`, which `{owner}` declares, and does not import \
                 `{owner}`: a module sees its own declarations and its imports' and nothing else"
            ));
        }
        let arguments: Vec<&TypeAnn> = annotation.arguments.iter().collect();
        check_names(module, &arguments, variables, visible, owners)?;
    }
    Ok(())
}

/// The whole library as one program's source: every module's imports at the
/// head, as one source unit holds them, then every module's declarations in
/// loading order.
#[must_use]
pub fn source() -> String {
    let mut heads: Vec<&str> = Vec::new();
    let mut bodies = String::new();
    for (_, module) in MODULES {
        for line in module.lines() {
            if line.starts_with("import ") {
                if !heads.contains(&line) {
                    heads.push(line);
                }
            } else {
                bodies.push_str(line);
                bodies.push('\n');
            }
        }
    }
    let mut source = heads.join("\n");
    source.push('\n');
    source.push_str(&bodies);
    source
}

/// Check every declaration in the library.
///
/// # Errors
///
/// Returns the first declaration that does not hold, which is a defect of the
/// binary rather than of any program.
pub fn check() -> Result<(), Diagnostic> {
    let mut graph = MODULES.to_vec();
    graph.extend_from_slice(WRITTEN);
    closure(&graph).map_err(|message| Diagnostic::name(0..0, message))?;
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
                    default: field
                        .default
                        .as_ref()
                        .and_then(crate::declarations::literal),
                    validator: None,
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
    fn every_module_of_the_library_is_closed_over_its_imports() {
        let mut graph = MODULES.to_vec();
        graph.extend_from_slice(WRITTEN);
        assert_eq!(closure(&graph), Ok(()));
    }

    #[test]
    fn a_module_sees_its_own_declarations_and_its_imports_and_nothing_else() {
        let base = ("base", "type Thing\ntrait Shape {}\n");
        // Neither declared here nor imported: refused, naming the owner.
        let refused = closure(&[base, ("user", "type Box { inner : Thing }\n")]).unwrap_err();
        assert!(
            refused.contains("`std/user` writes `Thing`, which `base` declares"),
            "{refused}"
        );
        // A bound, a parameter, an `impl` header and a receiver pattern are
        // written names as much as a field is.
        for source in [
            "fn f<T : Shape>(x: T) : T\n",
            "fn f(x: Thing) : Unit\n",
            "impl Shape for Int\n",
            "module for Thing\nfn f(self) : Unit\n",
        ] {
            assert!(closure(&[base, ("user", source)]).is_err(), "{source}");
        }
        // Imported, it resolves; a name no module declares is the language's.
        assert_eq!(
            closure(&[
                base,
                (
                    "user",
                    "import base\ntype Box { inner : Thing, any : Any }\n"
                )
            ]),
            Ok(())
        );
        // A type parameter shadows nothing it does not name.
        assert_eq!(
            closure(&[base, ("user", "type Box<Thing> { inner : Thing }\n")]),
            Ok(())
        );
    }

    #[test]
    fn the_import_graph_is_listed_in_order_and_has_no_cycle() {
        let later = closure(&[("a", "import b\n"), ("b", "type B\n")]).unwrap_err();
        assert!(later.contains("listed after it"), "{later}");
        let unknown = closure(&[("a", "import nowhere\n")]).unwrap_err();
        assert!(
            unknown.contains("no module of the standard library"),
            "{unknown}"
        );
        let child = closure(&[("a.b", "type B\n"), ("a", "import a.b\n")]).unwrap_err();
        assert!(child.contains("imports its own child"), "{child}");
        // The prelude is not a module: it may name any module of the graph.
        assert_eq!(
            closure(&[("prelude", "import a\n"), ("a", "type A\n")]),
            Ok(())
        );
        let twice = closure(&[("a", "type A\n"), ("b", "type A\n")]).unwrap_err();
        assert!(twice.contains("declared by both"), "{twice}");
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
