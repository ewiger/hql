//! Static checking: what a program's type is, before anything is evaluated.

use crate::ast::{
    Arg, Arm, Expr, ExternalDecl, FnDecl, ImplDecl, Kind, Pattern, Program, Stmt, TraitDecl,
};
use crate::builtins;
use crate::declarations::Declarations;
use crate::diagnostics::Diagnostic;
use crate::execution::{Checked, Program as TypedProgram};
use crate::extensions::{CheckCx, Operation, Purity, Resolution};
use crate::types::{TypeRef, collections};
use crate::vault::Vault;
use std::collections::HashMap;
use std::ops::Range;

/// Infer the type of a whole program against a vault.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub(crate) fn check(program: &Program, vault: &Vault) -> Result<TypedProgram, Diagnostic> {
    // A whole program starts from the built-in lattice; a cell of a document
    // keeps what the cells before it declared.
    crate::types::program::reset();
    check_with(program, vault, &[])
}

/// Check a program with further names in scope, each at its type: the values
/// of a document's other cells.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub(crate) fn check_with<'a>(
    program: &'a Program,
    vault: &'a Vault,
    bound: &[(String, TypeRef)],
) -> Result<TypedProgram, Diagnostic> {
    let mut scope = globals(vault);
    scope.extend(bound.iter().cloned());
    let _hidden = crate::declarations::HiddenTypes::enter();
    Checker::new(vault, scope, Resolution::new(&vault.extensions)?).program(program)
}

/// Check every statement, continuing past one that fails.
///
/// A statement that does not check still binds its name, at the open tree
/// type, so the statements after it are reported on their own faults rather
/// than on a cascade from this one.
pub(crate) fn check_collecting(
    program: &Program,
    vault: &Vault,
) -> (Option<TypedProgram>, Vec<Diagnostic>) {
    crate::types::program::reset();
    check_collecting_with(program, vault, &[])
}

/// [`check_collecting`] with further names in scope.
pub(crate) fn check_collecting_with<'a>(
    program: &'a Program,
    vault: &'a Vault,
    bound: &[(String, TypeRef)],
) -> (Option<TypedProgram>, Vec<Diagnostic>) {
    let resolution = match Resolution::new(&vault.extensions) {
        Ok(resolution) => resolution,
        // A vault that cannot resolve its own configuration has nothing to
        // say about the program: every statement would fail for one reason.
        Err(diagnostic) => return (None, vec![diagnostic]),
    };
    let mut checker = Checker::new(
        vault,
        {
            let mut scope = globals(vault);
            scope.extend(bound.iter().cloned());
            scope
        },
        resolution,
    );
    checker.foresee(program);
    let _hidden = crate::declarations::HiddenTypes::enter();
    let mut found = Vec::new();
    let mut last = TypeRef::UNIT;
    for statement in &program.statements {
        match checker.statement(statement) {
            Ok(inferred) => last = inferred,
            Err(diagnostic) => {
                if let Stmt::Bind { name, .. } = statement {
                    checker.scope.insert(name.clone(), TypeRef::DATA);
                }
                found.push(diagnostic);
                last = TypeRef::UNIT;
            }
        }
    }
    if !found.is_empty() {
        return (None, found);
    }
    match checker.checked.program(program, last) {
        Ok(program) => (Some(program), found),
        Err(diagnostic) => (None, vec![diagnostic]),
    }
}

/// Bind the type variables in `parameter` from the type an argument has:
/// `Seq<T>` against `List<Int>` binds `T` to `Int`. A variable bound twice
/// keeps the wider of the two.
fn instantiate(
    parameter: &TypeRef,
    actual: &TypeRef,
    bound: &mut HashMap<crate::types::TypeConstructor, TypeRef>,
) {
    if parameter.constructor.0.starts_with('%') {
        match bound.get(&parameter.constructor) {
            Some(held) if actual.is(held) => {}
            Some(held) if held.is(actual) => {
                bound.insert(parameter.constructor, actual.clone());
            }
            Some(_) => {}
            None => {
                bound.insert(parameter.constructor, actual.clone());
            }
        }
        return;
    }
    if !parameter.has_variable() {
        return;
    }
    let Some(view) = crate::types::builtin::system().ok().and_then(|system| {
        system
            .as_supertype(actual, parameter.constructor)
            .ok()
            .flatten()
    }) else {
        return;
    };
    for (inner, held) in parameter.args.iter().zip(&view.args) {
        instantiate(inner, held, bound);
    }
}

/// The type variables a receiver pattern holds, with the names they were
/// written under: `T` in `Seq<T>`.
fn pattern_variables(pattern: &TypeRef) -> Vec<(String, TypeRef)> {
    let mut found = Vec::new();
    if pattern.constructor.0.starts_with('%') {
        found.push((pattern.constructor.to_string(), pattern.clone()));
    }
    for arg in &pattern.args {
        found.extend(pattern_variables(arg));
    }
    found
}

/// Bind a record's parameters from the type a field's value has: `List<T>`
/// against `List<Int>` binds `T` to `Int`. A parameter met twice keeps the
/// wider type; a shape that does not match answers `false`.
fn unify(
    declared: &TypeRef,
    actual: &TypeRef,
    parameters: &[&'static str],
    bound: &mut HashMap<&'static str, TypeRef>,
) -> bool {
    if declared.args.is_empty()
        && let Some(parameter) = parameters
            .iter()
            .find(|parameter| **parameter == declared.constructor.0)
    {
        match bound.get(parameter) {
            Some(held) if actual.is(held) => {}
            Some(held) if held.is(actual) => {
                bound.insert(parameter, actual.clone());
            }
            Some(held) => {
                let Some(common) = held.common_type(actual) else {
                    return false;
                };
                bound.insert(parameter, common);
            }
            None => {
                bound.insert(parameter, actual.clone());
            }
        }
        return true;
    }
    if !crate::declarations::mentions_parameter(declared, parameters) {
        return actual.is(declared);
    }
    let Some(view) = crate::types::builtin::system().ok().and_then(|system| {
        system
            .as_supertype(actual, declared.constructor)
            .ok()
            .flatten()
    }) else {
        return false;
    };
    view.args.len() == declared.args.len()
        && declared
            .args
            .iter()
            .zip(&view.args)
            .all(|(inner, held)| unify(inner, held, parameters, bound))
}

/// A written type with each constructor parameter's arguments dropped:
/// `F<T>` is the variable `F`.
fn erase_constructors(written: &crate::ast::TypeAnn, constructors: &[&str]) -> crate::ast::TypeAnn {
    crate::ast::TypeAnn {
        name: written.name.clone(),
        arguments: if constructors.contains(&written.name.as_str()) {
            Vec::new()
        } else {
            written
                .arguments
                .iter()
                .map(|argument| erase_constructors(argument, constructors))
                .collect()
        },
        span: written.span.clone(),
    }
}

/// A function named in `UpperCamelCase` is a type's constructor: its name
/// is a concrete type the same source unit declares, and it returns that
/// type or a subtype of it. An alias states no result, so it is never one.
fn constructor_name(
    declarations: &Declarations,
    name: &str,
    result: Option<&crate::ast::TypeAnn>,
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    if !crate::constructors::is_constructor_name(name) {
        return Ok(());
    }
    let refused = |why: String| {
        Err(Diagnostic::typing(
            span.clone(),
            format!(
                "`{name}` is named like a type's constructor, {why}; name the function in \
                 snake_case"
            ),
        ))
    };
    match declarations.declares(name) {
        None => refused(format!(
            "and `{name}` names no type declared here: a constructor is declared beside its type"
        )),
        Some(true) => refused(format!(
            "and `{name}` is abstract, so it has no constructor"
        )),
        Some(false) => {
            let Some(result) = result else {
                return refused("and an alias states no result type".to_owned());
            };
            let builds = result.name == name
                || crate::types::builtin::constructor(&result.name)
                    .zip(crate::types::builtin::constructor(name))
                    .zip(crate::types::builtin::system().ok())
                    .is_some_and(|((built, own), system)| {
                        let arguments = vec![TypeRef::DATA; result.arguments.len()];
                        system
                            .as_supertype(&built.apply(arguments), own)
                            .ok()
                            .flatten()
                            .is_some()
                    });
            if builds {
                Ok(())
            } else {
                refused(format!(
                    "and it returns {}, which is not a `{name}`: a constructor returns its type",
                    result.name
                ))
            }
        }
    }
}

/// What a refused annotation could have written instead: a binding never
/// converts, so a different container is constructed and narrower elements
/// are selected.
fn conversion(inferred: &TypeRef, declared: &TypeRef) -> String {
    let (Some(has), Some(wants)) = (inferred.element(), declared.element()) else {
        return String::new();
    };
    if has.is(&wants) && declared.is_sequence() && !inferred.is_sequence() {
        return "; a set has no order to keep, so sort it: `| sort`".to_owned();
    }
    if has.is(&wants) && declared.is_concrete() {
        return format!(
            "; a binding never converts, so construct it: `{}(…)`",
            declared.constructor
        );
    }
    if wants.is(&has) {
        return format!("; to keep only the {wants} elements, select them with `of_type({wants})`");
    }
    String::new()
}

/// The type two values combined by a combiner have.
fn combined(combiner: &str, left: &TypeRef, right: &TypeRef) -> Result<TypeRef, String> {
    let joined = if left.is(right) {
        right.clone()
    } else if right.is(left) {
        left.clone()
    } else {
        return Err(format!(
            "`{combiner}` combines two values of one type, not {left} and {right}"
        ));
    };
    let fits = match combiner {
        "min" | "max" => joined.is_orderable(),
        "union" => joined.constructor == collections::SET || joined == TypeRef::NEVER,
        "merge" => {
            crate::types::MapKind::of(joined.constructor).is_some() || joined == TypeRef::DATA
        }
        "concat" => joined.is_sequence(),
        _ => false,
    };
    if !fits {
        let wanted = match combiner {
            "min" | "max" => "two values of one Orderable type",
            "union" => "two sets",
            "merge" => "two maps",
            _ => "two sequences",
        };
        return Err(format!("`{combiner}` combines {wanted}, not {joined}"));
    }
    Ok(match combiner {
        "concat" => TypeRef::list(joined.element().unwrap_or(TypeRef::NEVER)),
        _ => joined,
    })
}

/// A type with an alias's parameter placeholders replaced by the types they
/// took.
fn substituted_placeholders(
    ty: &TypeRef,
    fixed: &HashMap<crate::types::TypeConstructor, TypeRef>,
) -> TypeRef {
    if let Some(held) = fixed.get(&ty.constructor) {
        return held.clone();
    }
    ty.constructor.apply(
        ty.args
            .iter()
            .map(|arg| substituted_placeholders(arg, fixed)),
    )
}

/// A type with its variables replaced by what a call bound them to; one the
/// call left unbound is `Never`, as the element of an empty list is.
fn substituted(ty: &TypeRef, bound: &HashMap<crate::types::TypeConstructor, TypeRef>) -> TypeRef {
    if ty.constructor.0.starts_with('%') {
        return bound
            .get(&ty.constructor)
            .cloned()
            .unwrap_or(TypeRef::NEVER);
    }
    ty.constructor
        .apply(ty.args.iter().map(|arg| substituted(arg, bound)))
}

/// What a bound's arguments take from one implementation that applies: the
/// call's variables they leave open, filled from the implementation's
/// arguments, or `None` when the two disagree. The arguments the call
/// already decided are matched first, so they fix the implementation's
/// open parameters before the rest is read from it.
fn fill_from(
    wanted: &[TypeRef],
    applies: &Applies,
    bound: &HashMap<crate::types::TypeConstructor, TypeRef>,
) -> Option<HashMap<crate::types::TypeConstructor, TypeRef>> {
    let decided = |argument: &TypeRef| {
        !mentions_any(argument, &|constructor| {
            constructor.0.starts_with('%') && !bound.contains_key(&constructor)
        })
    };
    let mut open = HashMap::new();
    for (argument, given) in wanted.iter().zip(&applies.arguments) {
        if decided(argument) {
            let argument = substituted_placeholders(argument, bound);
            if !unify(given, &argument, &applies.open, &mut open) {
                return None;
            }
        }
    }
    let open: HashMap<crate::types::TypeConstructor, TypeRef> = open
        .into_iter()
        .map(|(parameter, ty)| (crate::types::TypeConstructor(parameter), ty))
        .collect();
    let mut fill = HashMap::new();
    for (argument, given) in wanted.iter().zip(&applies.arguments) {
        if decided(argument) {
            continue;
        }
        let given = substituted_placeholders(given, &open);
        if crate::declarations::mentions_parameter(&given, &applies.open) {
            return None;
        }
        instantiate(argument, &given, &mut fill);
    }
    fill.retain(|variable, _| !bound.contains_key(variable));
    Some(fill)
}

/// Whether a type or one of its arguments has a constructor the test holds
/// for.
fn mentions_any(ty: &TypeRef, test: &dyn Fn(crate::types::TypeConstructor) -> bool) -> bool {
    test(ty.constructor) || ty.args.iter().any(|arg| mentions_any(arg, test))
}

/// The names the program module starts with: the host's bindings, `vault`
/// among them, and the program module's bindings over it, `cards` among
/// them. No module sees any of them.
fn globals(vault: &Vault) -> HashMap<String, TypeRef> {
    let mut scope = base(vault);
    for (name, library) in library(vault) {
        scope.insert(name, library.result);
    }
    scope
}

/// What a host seeds before the library: `vault`, when there is one.
fn base(vault: &Vault) -> HashMap<String, TypeRef> {
    let mut scope: HashMap<String, TypeRef> = crate::extensions::io::BINDINGS
        .iter()
        .map(|(name, endpoint)| ((*name).to_owned(), endpoint.type_of()))
        .collect();
    // A document's cell is the one `hql render` is running.
    if matches!(crate::io::policy(), crate::io::Policy::Document { .. }) {
        scope.insert("cell".to_owned(), TypeRef::CELL);
    }
    if vault.is_present() {
        scope.insert(VAULT.to_owned(), TypeRef::VAULT);
    }
    scope
}

/// The program module's bindings over `vault`, each checked as its own
/// program in a scope that holds only the host's. Without a vault there are
/// none.
pub(crate) fn library(vault: &Vault) -> Vec<(String, TypedProgram)> {
    if !vault.is_present() {
        return Vec::new();
    }
    crate::library::bindings()
        .into_iter()
        .filter_map(|(name, value)| {
            let program = Program {
                statements: vec![Stmt::Expr(value)],
            };
            let typed = Checker::new(vault, base(vault), Resolution::new(&vault.extensions).ok()?)
                .program(&program)
                .ok()?;
            Some((name, typed))
        })
        .collect()
}

/// The operation types `std/core.hql` reserves, one per shape and their
/// common supertype: a program may not declare a type of these names.
const RESERVED_TYPES: [&str; 5] = [
    "Operation",
    "Elementwise",
    "Partition",
    "Reduction",
    "Barrier",
];

/// Whether a declaration of a reserved name is the one `std/core.hql`
/// makes, `abstract type Elementwise<I, O> : Operation<I, O>`: the library's
/// own text checks as a program, and restating it declares nothing new.
fn restates_operation_type(declaration: &crate::ast::TypeDecl) -> bool {
    let over_input_and_output = |written: &[crate::ast::TypeAnn]| {
        written.len() == 2
            && written
                .iter()
                .zip(["I", "O"])
                .all(|(argument, name)| argument.name == name && argument.arguments.is_empty())
    };
    let parents_hold = match declaration.name.as_str() {
        "Operation" => declaration.supertypes.is_empty(),
        _ => matches!(
            declaration.supertypes.as_slice(),
            [parent] if parent.name == "Operation" && over_input_and_output(&parent.arguments)
        ),
    };
    declaration.abstract_type
        && declaration.private.is_none()
        && declaration.bounds.is_empty()
        && declaration.parameters == ["I", "O"]
        && declaration.fields.is_empty()
        && declaration.union.is_none()
        && parents_hold
}

/// Refuse a public declaration of a module whose signature mentions one of
/// the module's private types or traits: a parameter, a result, a bound, a
/// field or a supertype. A public signature would otherwise hand importers a
/// value of a type they cannot name.
fn public_signatures(program: &Program) -> Result<(), Diagnostic> {
    let private: std::collections::HashSet<&str> = program
        .statements
        .iter()
        .filter(|statement| matches!(statement, Stmt::Type(_) | Stmt::Trait(_)))
        .filter(|statement| statement.private().is_some())
        .filter_map(Stmt::declared_name)
        .collect();
    if private.is_empty() {
        return Ok(());
    }
    for statement in &program.statements {
        if statement.private().is_some() {
            continue;
        }
        let mut written: Vec<&crate::ast::TypeAnn> = Vec::new();
        let span = match statement {
            Stmt::Fn(declaration) => {
                written.extend(declaration.parameters.iter().map(|p| &p.annotation));
                written.push(&declaration.result);
                written.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                &declaration.span
            }
            Stmt::External(declaration) => {
                written.extend(declaration.parameters.iter().map(|p| &p.annotation));
                written.push(&declaration.result);
                written.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                written.extend(declaration.receiver.as_ref());
                &declaration.span
            }
            Stmt::Type(declaration) => {
                written.extend(declaration.supertypes.iter());
                written.extend(declaration.fields.iter().map(|field| &field.annotation));
                written.extend(declaration.union.iter().flatten());
                written.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                &declaration.span
            }
            Stmt::Trait(declaration) => {
                written.extend(declaration.bounds.iter().map(|(_, bound)| bound));
                for method in &declaration.methods {
                    written.extend(method.parameters.iter().map(|p| &p.annotation));
                    written.push(&method.result);
                }
                &declaration.span
            }
            Stmt::Bind {
                annotation: Some(annotation),
                name_span,
                ..
            } => {
                written.push(annotation);
                name_span
            }
            _ => continue,
        };
        let Some(named) = written
            .into_iter()
            .find_map(|written| mentioned(written, &private))
        else {
            continue;
        };
        return Err(Diagnostic::typing(
            span.clone(),
            format!(
                "`{}` is public and its signature mentions `{named}`, which is private: an \
                 importer would hold a value of a type it cannot name; mark `{}` private or \
                 make `{named}` public",
                statement.declared_name().unwrap_or_default(),
                statement.declared_name().unwrap_or_default()
            ),
        ));
    }
    Ok(())
}

/// The first of `names` a written type mentions, itself or in an argument.
fn mentioned<'n>(
    written: &crate::ast::TypeAnn,
    names: &std::collections::HashSet<&'n str>,
) -> Option<&'n str> {
    names.get(written.name.as_str()).copied().or_else(|| {
        written
            .arguments
            .iter()
            .find_map(|argument| mentioned(argument, names))
    })
}

/// The parameter a constructor passed as a function takes its value by.
pub(crate) const CONSTRUCTED: &str = "%constructed";

/// The current vault, seeded by the host.
pub(crate) const VAULT: &str = "vault";

struct Checker<'a> {
    vault: &'a Vault,
    scope: HashMap<String, TypeRef>,
    /// Which extensions this program has imported.
    resolution: Resolution,
    /// What this program has declared.
    declarations: Declarations,
    checked: Checked,
    /// Declared functions whose last parameter collects trailing arguments,
    /// with their parameter count.
    variadic: HashMap<String, usize>,
    /// The parameter names of each declared function, for a diagnostic
    /// that names the parameter an argument does not fit.
    parameter_names: HashMap<String, Vec<String>>,
    /// Each generic function's type parameters: the name, its type variable
    /// and its bound.
    generic_functions: HashMap<String, Vec<(String, TypeRef, Option<TypeRef>)>>,
    /// Each type a program declares constructors for, with the name each
    /// form is bound under, in the order declared: a call takes the first
    /// whose first parameter its first argument fits.
    constructor_forms: HashMap<String, Vec<String>>,
    /// The types an alias's unannotated parameters took, while its body is
    /// checked.
    placeholders: HashMap<crate::types::TypeConstructor, TypeRef>,
    /// The effectful operations the body being checked uses, in the order
    /// first written.
    uses: Vec<String>,
    /// Each declared function whose body is effectful, with what it uses.
    effectful: HashMap<String, Vec<String>>,
    /// Every trait in scope: the core's operator traits and the program's.
    traits: HashMap<String, TraitInfo>,
    /// The program's trait declarations, for the default bodies an
    /// implementation inherits.
    trait_decls: HashMap<String, &'a TraitDecl>,
    /// Every implementation in force: the standard library's native ones,
    /// the imported modules', and the unit's own.
    impls: Vec<ImplInfo>,
    /// The module this checker checks, or empty for a program: where an
    /// implementation it declares comes from.
    unit: String,
    /// The trait bounds in force while a generic body is checked: each type
    /// variable, the trait it satisfies and the trait's arguments.
    trait_bounds: Vec<Bounded>,
    /// Each generic function's trait bounds, checked at every call.
    trait_bounded: HashMap<String, Vec<Bounded>>,
    /// The workspace modules this unit imported, each under its local name.
    modules: Vec<(String, std::sync::Arc<crate::modules::WorkspaceModule>)>,
    /// Every workspace module loaded for the program, by canonical name,
    /// shared with the checkers of the modules it loads.
    module_cache: ModuleCache,
    /// The modules being loaded, outermost first, for the cycle diagnostic.
    loading: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Whether this unit is a module file: declarations only, and a receiver
    /// pattern allowed.
    module_file: bool,
    /// The receiver pattern of a `module for T` file.
    receiver: Option<TypeRef>,
    /// The pattern's type variables, in scope for the whole module file.
    pattern_variables: Option<crate::declarations::TypeVariables>,
    /// Whether a declaration or an expression has been seen, after which an
    /// import is out of place.
    seen_body: bool,
    /// The external declarations of this unit: signatures an extension
    /// implements, bound to nothing here.
    externals: HashMap<String, TypeRef>,
    /// This unit's own receiver functions, `fn f(self)` under `module for T`.
    own_receivers: Vec<(String, TypeRef)>,
    /// This unit's own functions that take no `self`: a receiver call of one
    /// of their names never reaches an import's.
    own_functions: std::collections::HashSet<String>,
    /// The functions this unit declares below the statement being checked:
    /// a use of one of their names reaches neither the declaration, not yet
    /// checked, nor an import's, which the declaration wins over.
    declared_below: std::collections::HashSet<String>,
    /// This unit's own functions, by name, with their parameters and body:
    /// what binding one at `Elementwise<I, O>` verifies.
    function_bodies: HashMap<String, (Vec<String>, &'a Expr)>,
}

/// Type variables in scope, each by the name it is written as.
type Variables = Vec<(String, TypeRef)>;

/// A declared type parameter: the variable, and the type its arguments meet.
type Parameter = (TypeRef, Option<TypeRef>);

type ModuleCache = std::sync::Arc<
    std::sync::Mutex<HashMap<String, std::sync::Arc<crate::modules::WorkspaceModule>>>,
>;

/// A trait's contract: its type parameters and its functions, with `Self`
/// standing for the implementing type.
#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    /// The type variables its functions are written over, `T` of
    /// `Index<T>`, each with the bound an argument meets.
    pub parameters: Vec<(TypeRef, Option<TypeRef>)>,
    pub methods: Vec<MethodSig>,
}

/// A type parameter's trait bound, `E : Index<T>`: the parameter, its type
/// variable, the trait and the trait's arguments.
#[derive(Debug, Clone)]
pub(crate) struct Bounded {
    pub name: String,
    pub variable: TypeRef,
    pub trait_name: String,
    pub arguments: Vec<TypeRef>,
}

/// One way a type satisfies a trait: the trait's arguments it gives, and
/// the implementation's parameters the type left open, `T` of
/// `impl<T> Encoder<T> for Json`.
#[derive(Debug, Clone)]
struct Applies {
    arguments: Vec<TypeRef>,
    open: Vec<&'static str>,
}

/// One function of a trait.
#[derive(Debug, Clone)]
pub(crate) struct MethodSig {
    pub name: String,
    /// The parameters after `self`, written over `Self`.
    pub parameters: Vec<TypeRef>,
    pub result: TypeRef,
}

/// One implementation of a trait for a type: its own type parameters, the
/// trait's arguments and the implementing type, written over them.
#[derive(Debug, Clone)]
pub(crate) struct ImplInfo {
    pub trait_name: String,
    pub parameters: Vec<&'static str>,
    pub arguments: Vec<TypeRef>,
    pub target: TypeRef,
    pub span: Range<usize>,
    /// Where it is declared: `std/core` for a native one, a module's name,
    /// or empty for the program's own.
    pub origin: String,
}

impl ImplInfo {
    /// `Index<T> for Seq<T>`, as an error names it.
    fn written(&self) -> String {
        format!(
            "{} for {}",
            trait_written(&self.trait_name, &self.arguments),
            self.target
        )
    }

    /// Whether one assignment of the two implementations' parameters makes
    /// them one: the same trait at the same arguments for the same type.
    fn overlaps(&self, other: &Self) -> bool {
        self.trait_name == other.trait_name
            && could_equal(
                &self.target,
                &self.parameters,
                &other.target,
                &other.parameters,
            )
            && self.arguments.len() == other.arguments.len()
            && self
                .arguments
                .iter()
                .zip(&other.arguments)
                .all(|(mine, theirs)| {
                    could_equal(mine, &self.parameters, theirs, &other.parameters)
                })
    }

    fn whence(&self) -> String {
        if self.origin.is_empty() {
            format!("this program, at byte {}", self.span.start)
        } else if self.origin.starts_with("std/") {
            format!("`{}`, natively", self.origin)
        } else {
            format!("module `{}`", self.origin)
        }
    }
}

/// The native implementations the standard library declares, bodyless
/// `impl` headers an extension supplies the functions of: in force in
/// every program, as an implementation the program wrote would be.
pub(crate) fn native_impls() -> Vec<ImplInfo> {
    crate::library::native_impls()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|header| {
            let (parameters, variables) = impl_variables(&header.generics)?;
            let _variables = crate::declarations::TypeVariables::enter(&variables);
            Some(ImplInfo {
                target: crate::declarations::annotation(&header.target_type).ok()?,
                arguments: header
                    .arguments
                    .iter()
                    .map(|argument| crate::declarations::annotation(argument).ok())
                    .collect::<Option<_>>()?,
                parameters,
                trait_name: header.trait_name,
                span: 0..0,
                origin: format!("std/{}", header.module),
            })
        })
        .collect()
}

/// An implementation's type parameters: the names its types are written
/// over, and each as the type variable it stands for.
fn impl_variables(generics: &[String]) -> Option<(Vec<&'static str>, Variables)> {
    let variables = generics
        .iter()
        .map(|name| {
            Some((
                name.clone(),
                crate::types::builtin::type_variable(name, None)?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let parameters = variables
        .iter()
        .map(|(_, variable)| variable.constructor.0)
        .collect();
    Some((parameters, variables))
}

/// `Index<T>`, or `Eq` for a trait without parameters.
fn trait_written(trait_name: &str, arguments: &[TypeRef]) -> String {
    if arguments.is_empty() {
        return trait_name.to_owned();
    }
    format!(
        "{trait_name}<{}>",
        arguments
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Whether some assignment of the parameters on each side makes two types
/// the same.
fn could_equal(
    left: &TypeRef,
    left_parameters: &[&'static str],
    right: &TypeRef,
    right_parameters: &[&'static str],
) -> bool {
    let is_parameter = |ty: &TypeRef, parameters: &[&'static str]| {
        ty.args.is_empty() && parameters.contains(&ty.constructor.0)
    };
    if is_parameter(left, left_parameters) || is_parameter(right, right_parameters) {
        return true;
    }
    left.constructor == right.constructor
        && left.args.len() == right.args.len()
        && left
            .args
            .iter()
            .zip(&right.args)
            .all(|(left, right)| could_equal(left, left_parameters, right, right_parameters))
}

/// The type `Self` stands for inside a trait, before an implementation
/// binds it.
pub(crate) const SELF: crate::types::TypeConstructor = crate::types::TypeConstructor("%Self");

/// A type written over `Self` and a trait's parameters, with the
/// implementing type and the trait's arguments in their places.
fn in_place(
    written: &TypeRef,
    target: &TypeRef,
    parameters: &[(TypeRef, Option<TypeRef>)],
    arguments: &[TypeRef],
) -> TypeRef {
    if written.constructor == SELF {
        return target.clone();
    }
    if written.args.is_empty()
        && let Some(position) = parameters
            .iter()
            .position(|(parameter, _)| *parameter == *written)
        && let Some(argument) = arguments.get(position)
    {
        return argument.clone();
    }
    written.constructor.apply(
        written
            .args
            .iter()
            .map(|arg| in_place(arg, target, parameters, arguments)),
    )
}

/// A trait's function with `Self` and the trait's parameters in place.
fn signature_in_place(
    signature: &MethodSig,
    target: &TypeRef,
    info: &TraitInfo,
    arguments: &[TypeRef],
) -> MethodSig {
    let place = |written: &TypeRef| in_place(written, target, &info.parameters, arguments);
    MethodSig {
        name: signature.name.clone(),
        parameters: signature.parameters.iter().map(place).collect(),
        result: place(&signature.result),
    }
}

/// The traits the language owns, in scope in every program: the operators
/// resolve through them, and `std/core.hql` restates them.
pub(crate) fn core_traits() -> HashMap<String, TraitInfo> {
    let self_type = SELF.apply([]);
    let binary = |name: &str, result: TypeRef| TraitInfo {
        parameters: Vec::new(),
        methods: vec![MethodSig {
            name: name.to_owned(),
            parameters: vec![self_type.clone()],
            result,
        }],
    };
    [
        ("Add", binary("add", self_type.clone())),
        ("Sub", binary("sub", self_type.clone())),
        ("Mul", binary("mul", self_type.clone())),
        ("Div", binary("div", self_type.clone())),
        ("Rem", binary("rem", self_type.clone())),
        ("Eq", binary("eq", TypeRef::BOOL)),
        ("Ord", binary("compare", TypeRef::ORDERING)),
        ("Index", index_trait()),
        (
            "Slice",
            TraitInfo {
                parameters: Vec::new(),
                methods: vec![MethodSig {
                    name: "slice".to_owned(),
                    parameters: vec![TypeRef::RANGE],
                    result: self_type.clone(),
                }],
            },
        ),
    ]
    .into_iter()
    .map(|(name, info)| (name.to_owned(), info))
    .collect()
}

/// `trait Index<T> { fn at(self, position: Int) : Option<T> }`: what `xs[i]`
/// resolves through over a type that is not a built-in sequence.
fn index_trait() -> TraitInfo {
    let element = crate::types::builtin::type_variable("T", None).expect("T is a type variable");
    TraitInfo {
        parameters: vec![(element.clone(), None)],
        methods: vec![MethodSig {
            name: "at".to_owned(),
            parameters: vec![TypeRef::INT],
            result: TypeRef::optional(element),
        }],
    }
}

/// The names a `match` pattern binds, added to `bound`.
fn pattern_names(pattern: &crate::ast::Pattern, bound: &mut Vec<String>) {
    use crate::ast::Pattern;
    match pattern {
        Pattern::Bind(name) => bound.push(name.clone()),
        Pattern::Some(inner) | Pattern::Ok(inner) | Pattern::Err(inner) => {
            pattern_names(inner, bound);
        }
        Pattern::Type(_, inner) => pattern_names(inner, bound),
        Pattern::Wildcard
        | Pattern::Int(_)
        | Pattern::Float(_)
        | Pattern::Str(_)
        | Pattern::Bool(_)
        | Pattern::None => {}
    }
}

/// The trait and method an arithmetic operator resolves through.
fn operator_trait(operator: crate::ast::Arith) -> (&'static str, &'static str) {
    use crate::ast::Arith;
    match operator {
        Arith::Add => ("Add", "add"),
        Arith::Sub => ("Sub", "sub"),
        Arith::Mul => ("Mul", "mul"),
        Arith::Div => ("Div", "div"),
        Arith::Rem => ("Rem", "rem"),
    }
}

impl<'a> Checker<'a> {
    /// A checker for one import unit, starting from a scope and the modules
    /// visible before the unit's own imports.
    fn new(vault: &'a Vault, scope: HashMap<String, TypeRef>, resolution: Resolution) -> Self {
        Checker {
            vault,
            scope,
            resolution,
            declarations: Declarations::default(),
            checked: Checked::default(),
            variadic: HashMap::new(),
            parameter_names: HashMap::new(),
            generic_functions: HashMap::new(),
            constructor_forms: HashMap::new(),
            placeholders: HashMap::new(),
            uses: Vec::new(),
            effectful: HashMap::new(),
            traits: core_traits(),
            trait_decls: HashMap::new(),
            impls: native_impls(),
            unit: String::new(),
            trait_bounds: Vec::new(),
            trait_bounded: HashMap::new(),
            modules: Vec::new(),
            module_cache: ModuleCache::default(),
            loading: std::sync::Arc::default(),
            module_file: false,
            receiver: None,
            pattern_variables: None,
            seen_body: false,
            externals: HashMap::new(),
            own_receivers: Vec::new(),
            own_functions: std::collections::HashSet::new(),
            declared_below: std::collections::HashSet::new(),
            function_bodies: HashMap::new(),
        }
    }

    /// Note the functions the unit declares before any is checked, so a use
    /// written above one of them is told apart from a use of an import.
    fn foresee(&mut self, program: &Program) {
        self.declared_below = program
            .statements
            .iter()
            .filter(|statement| matches!(statement, Stmt::Fn(_) | Stmt::Alias(_)))
            .filter_map(Stmt::declared_name)
            .map(str::to_owned)
            .collect();
    }

    fn program(&mut self, program: &'a Program) -> Result<TypedProgram, Diagnostic> {
        self.foresee(program);
        let mut last = TypeRef::UNIT;
        for statement in &program.statements {
            last = self.statement(statement)?;
        }
        self.checked.program(program, last)
    }

    /// Check a function declaration, and record it as effectful when its body
    /// uses an effectful operation, directly or through another function.
    fn declaring(
        &mut self,
        name: &str,
        declare: impl FnOnce(&mut Self) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        let outer = std::mem::take(&mut self.uses);
        let declared = declare(self);
        let used = std::mem::replace(&mut self.uses, outer);
        declared?;
        if used.is_empty() {
            self.effectful.remove(name);
        } else {
            self.effectful.insert(name.to_owned(), used.clone());
            self.checked.effects.push((name.to_owned(), used));
        }
        Ok(())
    }

    /// Bind a user operation: a lambda or a function of the program's own
    /// at one of the operation types. Its value must be the function the
    /// type is, `I -> O`, and an `Elementwise` one must read its one
    /// element and nothing else. It is effectful when its body is.
    fn operation(
        &mut self,
        name: &str,
        declared: &TypeRef,
        value: &'a Expr,
    ) -> Result<(), Diagnostic> {
        let mut inferred = TypeRef::UNIT;
        self.declaring(name, |checker| {
            inferred = checker.expecting(value, Some(declared))?;
            Ok(())
        })?;
        if inferred.is(declared) {
            if let Kind::Ident(held) = &value.kind
                && let Some(uses) = self.effectful.get(held).cloned()
            {
                self.effectful.insert(name.to_owned(), uses);
            }
            return Ok(());
        }
        let function = declared.operation_function().ok_or_else(|| {
            Diagnostic::typing(value.span.clone(), format!("{declared} is not a function"))
        })?;
        if !inferred.is(&function) {
            return Err(Diagnostic::typing(
                value.span.clone(),
                format!(
                    "`{name}` is declared {declared}, a function {function}, but its value is \
                     {inferred}"
                ),
            ));
        }
        if declared.operation_shape() != Some(crate::extensions::Shape::Elementwise) {
            if let Kind::Ident(held) = &value.kind
                && let Some(uses) = self.effectful.get(held).cloned()
            {
                self.effectful.insert(name.to_owned(), uses);
            }
            return Ok(());
        }
        let (parameters, body) = match &value.kind {
            Kind::Lambda {
                parameters, body, ..
            } => (parameters.clone(), body.as_ref()),
            Kind::Ident(held) => match self.function_bodies.get(held) {
                Some((parameters, body)) => {
                    if let Some(uses) = self.effectful.get(held).cloned() {
                        self.effectful.insert(name.to_owned(), uses);
                    }
                    (parameters.clone(), *body)
                }
                None => {
                    return Err(Diagnostic::typing(
                        value.span.clone(),
                        format!(
                            "`{name}` is declared {declared}, and the body of `{held}` is not \
                             this program's to verify reads its one element only"
                        ),
                    ));
                }
            },
            _ => {
                return Err(Diagnostic::typing(
                    value.span.clone(),
                    format!(
                        "`{name}` is declared {declared}: its value is a lambda or a function \
                         of this program, whose body is verified to read its one element only"
                    ),
                ));
            }
        };
        let mut bound: Vec<String> = parameters;
        if let Some((read, at)) = self.read_beyond(body, &mut bound) {
            return Err(Diagnostic::typing(
                at,
                format!(
                    "`{name}` is declared {declared}, so its body reads its one element and \
                     nothing else, and this reads `{read}`"
                ),
            ));
        }
        Ok(())
    }

    /// The first thing a body reads besides the names bound within it: a
    /// value bound outside it, or a document by `@[[name]]`. A function or
    /// a type named outside it is not a read; calling one is.
    fn read_beyond(&self, body: &Expr, bound: &mut Vec<String>) -> Option<(String, Range<usize>)> {
        let depth = bound.len();
        let found = self.read_beyond_inner(body, bound);
        bound.truncate(depth);
        found
    }

    fn read_beyond_inner(
        &self,
        body: &Expr,
        bound: &mut Vec<String>,
    ) -> Option<(String, Range<usize>)> {
        match &body.kind {
            Kind::Ident(name) => {
                let outside = !bound.iter().any(|held| held == name)
                    && self
                        .scope
                        .get(name)
                        .is_some_and(|ty| ty.as_function().is_none());
                outside.then(|| (name.clone(), body.span.clone()))
            }
            Kind::DocRef(name) => Some((format!("@[[{name}]]"), body.span.clone())),
            Kind::Int(_) | Kind::Float(_) | Kind::Bool(_) | Kind::Str(_) => None,
            Kind::List(values) | Kind::Set(values) => self.first_read(values.iter(), bound),
            Kind::Construct { arguments, .. } => {
                self.first_read(arguments.iter().map(|argument| &argument.value), bound)
            }
            Kind::Data(entries) => self.first_read(entries.iter().map(|(_, value)| value), bound),
            Kind::Arith(_, left, right)
            | Kind::Compare { left, right, .. }
            | Kind::Relate { left, right, .. }
            | Kind::Logic(_, left, right)
            | Kind::Index(left, right) => self.first_read([left.as_ref(), right.as_ref()], bound),
            Kind::Not(inner) | Kind::Field(inner, _) => self.read_beyond(inner, bound),
            Kind::Slice(value, start, end) => self.first_read(
                std::iter::once(value.as_ref())
                    .chain(start.as_deref())
                    .chain(end.as_deref()),
                bound,
            ),
            Kind::Link {
                source,
                target,
                data,
            } => self.first_read(
                [source.as_ref(), target.as_ref()]
                    .into_iter()
                    .chain(data.as_deref()),
                bound,
            ),
            // A step or a function named in call position is not a read of
            // a value; its arguments are.
            Kind::Pipe(input, step) => {
                self.read_beyond(input, bound).or_else(|| match &step.kind {
                    Kind::Call { callee, arguments } => {
                        self.callee_read(callee, bound).or_else(|| {
                            self.first_read(arguments.iter().map(|argument| &argument.value), bound)
                        })
                    }
                    Kind::Tee { branch, .. } => self.read_beyond(branch, bound),
                    _ => self.callee_read(step, bound),
                })
            }
            Kind::Call { callee, arguments } => self.callee_read(callee, bound).or_else(|| {
                self.first_read(arguments.iter().map(|argument| &argument.value), bound)
            }),
            Kind::Lambda {
                parameters, body, ..
            } => {
                bound.extend(parameters.iter().cloned());
                self.read_beyond(body, bound)
            }
            Kind::Match { subject, arms } => self.read_beyond(subject, bound).or_else(|| {
                arms.iter().find_map(|arm| {
                    let depth = bound.len();
                    pattern_names(&arm.pattern, bound);
                    let found = arm
                        .guard
                        .iter()
                        .chain(std::iter::once(&arm.body))
                        .find_map(|expression| self.read_beyond(expression, bound));
                    bound.truncate(depth);
                    found
                })
            }),
            Kind::Tee { branch, .. } => self.read_beyond(branch, bound),
            Kind::Given {
                value,
                condition,
                otherwise,
            } => self.first_read(
                [value.as_ref(), condition.as_ref()]
                    .into_iter()
                    .chain(otherwise.as_deref()),
                bound,
            ),
            Kind::Block(statements) => self.block_read(statements, bound),
        }
    }

    /// What a callee reads: nothing when it names a function or a step, the
    /// receiver when it is a field of one.
    fn callee_read(
        &self,
        callee: &Expr,
        bound: &mut Vec<String>,
    ) -> Option<(String, Range<usize>)> {
        match &callee.kind {
            Kind::Ident(_) => None,
            Kind::Field(receiver, _) if matches!(&receiver.kind, Kind::Ident(module) if self.is_module(module)) => {
                None
            }
            _ => self.read_beyond(callee, bound),
        }
    }

    fn first_read<'e>(
        &self,
        expressions: impl IntoIterator<Item = &'e Expr>,
        bound: &mut Vec<String>,
    ) -> Option<(String, Range<usize>)> {
        expressions
            .into_iter()
            .find_map(|expression| self.read_beyond(expression, bound))
    }

    fn block_read(
        &self,
        statements: &[crate::ast::BlockStmt],
        bound: &mut Vec<String>,
    ) -> Option<(String, Range<usize>)> {
        use crate::ast::BlockStmt;
        for statement in statements {
            let found = match statement {
                BlockStmt::Bind { name, value } => {
                    let found = self.read_beyond(value, bound);
                    bound.push(name.clone());
                    found
                }
                BlockStmt::Yield(value) => self.read_beyond(value, bound),
                BlockStmt::For {
                    name,
                    iterable,
                    body,
                } => self.read_beyond(iterable, bound).or_else(|| {
                    let depth = bound.len();
                    bound.push(name.clone());
                    let found = self.block_read(body, bound);
                    bound.truncate(depth);
                    found
                }),
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Note that the body being checked uses an operation, when it is
    /// effectful: a step declared so, or a function whose body is.
    fn using(&mut self, name: &str, effectful: bool) {
        if effectful && !self.uses.iter().any(|held| held == name) {
            self.uses.push(name.to_owned());
        }
    }

    /// Warn where a name is introduced that departs from its kind's casing.
    fn lint(&mut self, kind: crate::naming::Kind, name: &str, span: &Range<usize>) {
        // A name the language makes for itself, a hole's parameter, cannot be
        // written and is nobody's to spell.
        if name.starts_with('%') {
            return;
        }
        if let Some(message) = crate::naming::departure(kind, name) {
            CheckCx::warn(self, crate::warnings::Warning::new(span.clone(), message));
        }
    }

    fn statement(&mut self, statement: &'a Stmt) -> Result<TypeRef, Diagnostic> {
        if !matches!(statement, Stmt::Import { .. } | Stmt::ModuleFor { .. }) {
            self.seen_body = true;
        }
        if let Some(marker) = statement.private()
            && !self.module_file
        {
            return Err(Diagnostic::syntax(
                marker.clone(),
                "`private` belongs to a module: every declaration of a program is public, \
                 since no other unit reads it",
            ));
        }
        if let Stmt::Fn(FnDecl { name, .. }) | Stmt::Alias(crate::ast::AliasDecl { name, .. }) =
            statement
        {
            self.declared_below.remove(name);
        }
        match statement {
            Stmt::Fn(declaration) if !declaration.has_self => {
                self.own_functions.insert(declaration.name.clone());
                self.function_bodies.insert(
                    declaration.name.clone(),
                    (
                        declaration
                            .parameters
                            .iter()
                            .map(|parameter| parameter.name.clone())
                            .collect(),
                        &declaration.body,
                    ),
                );
            }
            Stmt::Alias(declaration) => {
                self.own_functions.insert(declaration.name.clone());
            }
            _ => {}
        }
        if self.module_file && matches!(statement, Stmt::Expr(_)) {
            let Stmt::Expr(expression) = statement else {
                unreachable!("matched an expression");
            };
            return Err(Diagnostic::typing(
                expression.span.clone(),
                "a module declares functions and types; it has no expression to evaluate",
            ));
        }
        Ok({
            match statement {
                Stmt::Bind {
                    name,
                    name_span,
                    annotation,
                    value,
                    ..
                } => {
                    self.lint(crate::naming::Kind::Binding, name, name_span);
                    let expected = annotation
                        .as_ref()
                        .map(crate::declarations::annotation)
                        .transpose()?;
                    if let Some(declared) = expected.clone().filter(TypeRef::is_operation) {
                        self.operation(name, &declared, value)?;
                        self.checked
                            .views
                            .insert(value.span.clone(), declared.clone());
                        self.scope.insert(name.clone(), declared);
                        return Ok(TypeRef::UNIT);
                    }
                    let inferred = self.expecting(value, expected.as_ref())?;
                    if let Some(annotation) = annotation {
                        let declared = crate::declarations::annotation(annotation)?;
                        if !inferred.is(&declared) {
                            return Err(Diagnostic::typing(
                                value.span.clone(),
                                format!(
                                    "`{name}` is declared {declared} but its value is \
                                     {inferred}{}",
                                    conversion(&inferred, &declared)
                                ),
                            ));
                        }
                        self.checked
                            .views
                            .insert(value.span.clone(), declared.clone());
                        self.scope.insert(name.clone(), declared);
                    } else {
                        self.scope.insert(name.clone(), inferred);
                    }
                    TypeRef::UNIT
                }
                Stmt::Import { path, alias, span } => {
                    if self.seen_body {
                        return Err(Diagnostic::syntax(
                            span.clone(),
                            "an import sits at the head of a source unit, before its declarations \
                             and expressions",
                        ));
                    }
                    let dotted = path.join(".");
                    let local = alias
                        .clone()
                        .unwrap_or_else(|| path.last().cloned().unwrap_or_default());
                    if let Some((held, module)) =
                        self.modules.iter().find(|(held, _)| *held == local)
                        && module.name != dotted
                    {
                        return Err(Diagnostic::name(
                            span.clone(),
                            format!(
                                "`{held}` already names the module `{}` here; import `{dotted}` \
                                 under another name with `as`",
                                module.name
                            ),
                        ));
                    }
                    // A child may import its parent; the reverse would give
                    // the module tree a dependency in both directions.
                    if !self.unit.is_empty() && dotted.starts_with(&format!("{}.", self.unit)) {
                        CheckCx::warn(
                            self,
                            crate::warnings::Warning::new(
                                span.clone(),
                                format!(
                                    "`{}` imports its own child `{dotted}`: what a parent needs \
                                     from a child belongs in the parent, or in a module both \
                                     import",
                                    self.unit
                                ),
                            ),
                        );
                    }
                    if !self.resolution.import(&dotted, alias.as_deref(), span)? {
                        self.import_module(&dotted, &local, span)?;
                    }
                    TypeRef::UNIT
                }
                Stmt::ModuleFor { pattern, span } => {
                    // The receiver pattern of the declarations that follow. A
                    // module file opens with it; the standard library's
                    // modules, checked one after another, each open their own.
                    // A pattern is concrete or structural, never a bare
                    // variable: that would need coherence rules not written.
                    if pattern.arguments.is_empty()
                        && pattern.name.len() == 1
                        && crate::types::builtin::constructor(&pattern.name).is_none()
                    {
                        return Err(Diagnostic::typing(
                            span.clone(),
                            format!(
                                "`module for {}` matches every type; a receiver pattern is a \
                                 type or a constructor over variables, `Seq<T>`",
                                pattern.name
                            ),
                        ));
                    }
                    self.pattern_variables = None;
                    self.own_receivers.clear();
                    // A one-letter argument that names no type is a pattern
                    // variable, bound per call: `Seq<T>`.
                    let variables: Vec<(String, TypeRef)> = pattern
                        .arguments
                        .iter()
                        .filter(|argument| {
                            argument.arguments.is_empty()
                                && argument.name.len() == 1
                                && crate::types::builtin::constructor(&argument.name).is_none()
                        })
                        .filter_map(|argument| {
                            crate::types::builtin::type_variable(&argument.name, None)
                                .map(|variable| (argument.name.clone(), variable))
                        })
                        .collect();
                    self.pattern_variables =
                        Some(crate::declarations::TypeVariables::enter(&variables));
                    self.receiver = Some(crate::declarations::annotation(pattern)?);
                    TypeRef::UNIT
                }
                // A declaration is checked and yields nothing to show. It
                // introduces no value: constructing one is separate work.
                Stmt::Type(declaration) => {
                    use crate::naming::Kind as Name;
                    if RESERVED_TYPES.contains(&declaration.name.as_str())
                        && !restates_operation_type(declaration)
                    {
                        return Err(Diagnostic::typing(
                            declaration.span.clone(),
                            format!(
                                "`{}` is reserved: the operation types `Operation`, \
                                 `Elementwise`, `Partition`, `Reduction` and `Barrier` are \
                                 the standard library's, and a program declares none of them",
                                declaration.name
                            ),
                        ));
                    }
                    self.lint(Name::Type, &declaration.name, &declaration.span);
                    for parameter in &declaration.parameters {
                        self.lint(Name::TypeParameter, parameter, &declaration.span);
                    }
                    for field in &declaration.fields {
                        self.lint(Name::Field, &field.name, &field.span);
                    }
                    self.declarations.declare(declaration)?;
                    crate::declarations::enter_lattice(declaration)?;
                    TypeRef::UNIT
                }
                Stmt::Fn(declaration) => {
                    let bound = self.constructor_binding(declaration);
                    self.declaring(&bound, |checker| checker.function(declaration, &bound))?;
                    if bound != declaration.name {
                        self.constructor_form(declaration, &bound)?;
                    }
                    if declaration.has_self
                        && let Some(ty) = self.scope.get(&declaration.name).cloned()
                    {
                        self.own_receivers
                            .retain(|(held, _)| *held != declaration.name);
                        self.own_receivers.push((declaration.name.clone(), ty));
                    }
                    TypeRef::UNIT
                }
                Stmt::External(declaration) => {
                    self.external(declaration)?;
                    TypeRef::UNIT
                }
                Stmt::Alias(alias) => {
                    self.declaring(&alias.name, |checker| checker.alias(alias))?;
                    TypeRef::UNIT
                }
                Stmt::Trait(declaration) => {
                    self.declare_trait(declaration)?;
                    TypeRef::UNIT
                }
                Stmt::Impl(declaration) => {
                    self.implement(declaration)?;
                    TypeRef::UNIT
                }
                Stmt::Expr(expression) => self.expression(expression)?,
            }
        })
    }

    /// Check an expression whose type is known in advance, so a lambda written
    /// against a function type needs no annotations of its own.
    fn expecting(
        &mut self,
        expression: &Expr,
        expected: Option<&TypeRef>,
    ) -> Result<TypeRef, Diagnostic> {
        if let (Kind::Lambda { .. }, Some((parameters, _))) =
            (&expression.kind, expected.and_then(TypeRef::as_function))
        {
            let ty = self.lambda_value(expression, Some(parameters))?;
            self.checked
                .types
                .insert(expression.span.clone(), ty.clone());
            return Ok(ty);
        }
        // A collection literal checked against a collection type holds its
        // elements at that type: `[Circle(…), Square(…)]` is a `List<Shape>`
        // where the annotation says so, though the elements alone meet
        // nowhere.
        if let (Kind::List(values) | Kind::Set(values), Some(expected)) =
            (&expression.kind, expected)
            && let Some(element) = expected.element()
            && !element.has_variable()
            && !element.has_placeholder()
        {
            for value in values {
                let actual = self.expecting(value, Some(&element))?;
                if !actual.is(&element) {
                    return Err(Diagnostic::typing(
                        value.span.clone(),
                        format!("this element is {actual}, and the collection holds {element}"),
                    ));
                }
            }
            let ty = if matches!(expression.kind, Kind::Set(_)) {
                TypeRef::set(element)
            } else {
                TypeRef::list(element)
            };
            self.checked
                .types
                .insert(expression.span.clone(), ty.clone());
            return Ok(ty);
        }
        let inferred = self.expression(expression)?;
        Ok(self.decoding(expression, inferred, expected))
    }

    /// A read, `source` or `tap` of an endpoint, at the element type it is
    /// expected to give: `s : Seq<Sighting> = source(file("birds.json"))`
    /// decodes each `Data` document as a `Sighting`. A type a document can be
    /// read as is a declared record, or a type its document type narrows to;
    /// any other expectation leaves the read as it is, for the binding to
    /// refuse.
    fn decoding(
        &mut self,
        expression: &Expr,
        inferred: TypeRef,
        expected: Option<&TypeRef>,
    ) -> TypeRef {
        let Some(expected) = expected else {
            return inferred;
        };
        let step = match &expression.kind {
            Kind::Pipe(_, step) => self
                .checked
                .steps
                .get(&step.span)
                .or_else(|| self.checked.steps.get(&expression.span)),
            _ => self.checked.steps.get(&expression.span),
        };
        if !step.is_some_and(|step| crate::extensions::io::reads(step)) || inferred.is(expected) {
            return inferred;
        }
        let (Some(document), Some(wanted)) = (inferred.args.first(), expected.iterated()) else {
            return inferred;
        };
        if wanted.has_variable()
            || wanted.has_placeholder()
            || !crate::types::program::may_be(document, &wanted)
        {
            return inferred;
        }
        let decoded = inferred.constructor.apply([wanted]);
        if !decoded.is(expected) {
            return inferred;
        }
        self.checked
            .types
            .insert(expression.span.clone(), decoded.clone());
        decoded
    }

    /// A lambda as a value: its parameters are typed by their annotations or
    /// by the function type it is checked against, and its type is a function.
    fn lambda_value(
        &mut self,
        expression: &Expr,
        expected: Option<&[TypeRef]>,
    ) -> Result<TypeRef, Diagnostic> {
        let Kind::Lambda {
            parameters,
            annotations,
            body,
        } = &expression.kind
        else {
            return self.expression(expression);
        };
        if let Some(expected) = expected
            && expected.len() != parameters.len()
        {
            return Err(Diagnostic::typing(
                expression.span.clone(),
                format!(
                    "this lambda takes {} parameters where a function of {} is expected",
                    parameters.len(),
                    expected.len()
                ),
            ));
        }
        for name in parameters {
            self.lint(crate::naming::Kind::Parameter, name, &expression.span);
        }
        let mut types = Vec::new();
        for (index, (name, annotation)) in parameters.iter().zip(annotations).enumerate() {
            let declared = annotation
                .as_ref()
                .map(crate::declarations::annotation)
                .transpose()?;
            let given = expected.map(|expected| expected[index].clone());
            types.push(match (declared, given) {
                (Some(declared), Some(given)) if !given.is(&declared) => {
                    return Err(Diagnostic::typing(
                        expression.span.clone(),
                        format!("parameter `{name}` is declared {declared} but receives {given}"),
                    ));
                }
                (Some(declared), _) => declared,
                (None, Some(given)) => given,
                (None, None) if name.starts_with(crate::parser::HOLE) => self.hole(name, body)?,
                (None, None) => {
                    return Err(Diagnostic::typing(
                        expression.span.clone(),
                        format!(
                            "a lambda written as a value types its parameters: `({name}: Type) => …`"
                        ),
                    ));
                }
            });
        }
        let result = self
            .with_bindings(parameters.iter().cloned().zip(types.clone()), |checker| {
                checker.expression(body)
            })?;
        let ty = TypeRef::function(types, result).ok_or_else(|| {
            Diagnostic::typing(
                expression.span.clone(),
                "a function takes at most eight parameters",
            )
        })?;
        self.checked
            .functions
            .insert(expression.span.clone(), ty.clone());
        Ok(ty)
    }

    /// Run `check` with names bound, restoring what they shadowed afterwards.
    fn with_bindings<T>(
        &mut self,
        bindings: impl IntoIterator<Item = (String, TypeRef)>,
        check: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let previous = bindings
            .into_iter()
            .map(|(name, ty)| {
                let held = self.scope.insert(name.clone(), ty);
                (name, held)
            })
            .collect::<Vec<_>>();
        let result = check(self);
        for (name, held) in previous.into_iter().rev() {
            match held {
                Some(held) => {
                    self.scope.insert(name, held);
                }
                None => {
                    self.scope.remove(&name);
                }
            }
        }
        result
    }

    /// A declared function: bound before its body is checked, so it may call
    /// itself, and its body must have the declared result type.
    fn function(&mut self, declaration: &FnDecl, bound: &str) -> Result<(), Diagnostic> {
        constructor_name(
            &self.declarations,
            &declaration.name,
            Some(&declaration.result),
            &declaration.span,
        )?;
        if !crate::constructors::is_constructor_name(&declaration.name) {
            self.lint(
                crate::naming::Kind::Function,
                &declaration.name,
                &declaration.span,
            );
        }
        for parameter in &declaration.parameters {
            self.lint(
                crate::naming::Kind::Parameter,
                &parameter.name,
                &parameter.span,
            );
        }
        for generic in &declaration.generics {
            self.lint(
                crate::naming::Kind::TypeParameter,
                generic,
                &declaration.span,
            );
        }
        // A generic parameter stands for its type variable while the
        // declaration is checked; its bound is checked at each call.
        let mut variables = Vec::new();
        let mut traits_bounding = Vec::new();
        for name in &declaration.generics {
            let written = declaration
                .bounds
                .iter()
                .find(|(bounded, _)| bounded == name)
                .map(|(_, bound)| bound);
            // `T : Shape` names a trait: the variable is unbounded in the
            // lattice, and the trait is checked at each call and usable in
            // the body. Its arguments name the other parameters, so they are
            // read once every parameter is in scope.
            let trait_bound = written.filter(|bound| self.traits.contains_key(&bound.name));
            let bound = match (&trait_bound, written) {
                (None, Some(bound)) => Some(crate::declarations::annotation(bound)?),
                _ => None,
            };
            if let Some(trait_bound) = trait_bound
                && let Some(variable) = crate::types::builtin::type_variable(name, None)
            {
                traits_bounding.push((name.clone(), variable, trait_bound));
            }
            let variable =
                crate::types::builtin::type_variable(name, bound.as_ref()).ok_or_else(|| {
                    Diagnostic::typing(
                        declaration.span.clone(),
                        format!(
                            "a type parameter is one capital letter, such as `T`, not `{name}`"
                        ),
                    )
                })?;
            if variables
                .iter()
                .any(|(held, _, _): &(String, TypeRef, Option<TypeRef>)| held == name)
            {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!("the type parameter `{name}` is written twice"),
                ));
            }
            variables.push((name.clone(), variable, bound));
        }
        let in_scope: Vec<(String, TypeRef)> = variables
            .iter()
            .map(|(name, variable, _)| (name.clone(), variable.clone()))
            .collect();
        let _variables = crate::declarations::TypeVariables::enter(&in_scope);
        let traits_bounding = traits_bounding
            .into_iter()
            .map(|(name, variable, written)| {
                Ok(Bounded {
                    name,
                    variable,
                    trait_name: written.name.clone(),
                    arguments: self.trait_arguments(written)?,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        if variables.is_empty() {
            self.generic_functions.remove(bound);
        } else {
            self.generic_functions.insert(bound.to_owned(), variables);
        }
        if traits_bounding.is_empty() {
            self.trait_bounded.remove(bound);
        } else {
            self.trait_bounded
                .insert(bound.to_owned(), traits_bounding.clone());
        }
        let bounds_before = self.trait_bounds.len();
        self.trait_bounds.extend(traits_bounding);
        let checked = self.function_body(declaration, bound);
        self.trait_bounds.truncate(bounds_before);
        checked
    }

    /// The name a declared function is bound under: its own, or, for a
    /// constructor, one per form, so that several forms of one type are
    /// held side by side and a call selects among them.
    fn constructor_binding(&mut self, declaration: &FnDecl) -> String {
        if !crate::constructors::is_constructor_name(&declaration.name)
            || self.declarations.declares(&declaration.name).is_none()
        {
            return declaration.name.clone();
        }
        let forms = self
            .constructor_forms
            .get(&declaration.name)
            .map_or(0, Vec::len);
        let bound = format!("{}#{forms}", declaration.name);
        self.checked
            .bound_names
            .insert(declaration.span.clone(), bound.clone());
        bound
    }

    /// Record a constructor's form once its signature is checked: forms of
    /// one arity differ in their first parameter, so the first argument of
    /// a call selects one.
    fn constructor_form(&mut self, declaration: &FnDecl, bound: &str) -> Result<(), Diagnostic> {
        let signature = |checker: &Self, name: &str| {
            checker
                .scope
                .get(name)
                .and_then(TypeRef::as_function)
                .map(|(parameters, _)| parameters.to_vec())
        };
        let own = signature(self, bound).unwrap_or_default();
        let held = self
            .constructor_forms
            .get(&declaration.name)
            .cloned()
            .unwrap_or_default();
        for earlier in &held {
            let Some(parameters) = signature(self, earlier) else {
                continue;
            };
            if parameters.len() == own.len() && parameters.first() == own.first() {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!(
                        "`{}` already has a constructor taking {} first; the forms of a type \
                         differ in their first parameter",
                        declaration.name,
                        own.first()
                            .map_or_else(|| "nothing".to_owned(), ToString::to_string)
                    ),
                ));
            }
        }
        self.constructor_forms
            .entry(declaration.name.clone())
            .or_default()
            .push(bound.to_owned());
        Ok(())
    }

    /// The forms of a program's constructor, each with its parameters.
    fn constructor_signatures(&self, name: &str) -> Vec<(String, Vec<TypeRef>)> {
        if self.scope.contains_key(name) {
            return Vec::new();
        }
        self.constructor_forms
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|bound| {
                let (parameters, _) = self.scope.get(bound)?.as_function()?;
                Some((bound.clone(), parameters.to_vec()))
            })
            .collect()
    }

    /// A call of a program's constructor: fields by name build the record,
    /// and otherwise the first form whose first parameter the first
    /// argument fits is applied.
    fn construct_by_form(
        &mut self,
        callee: &Expr,
        name: &str,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        if !arguments.is_empty()
            && arguments.iter().all(|argument| argument.name.is_some())
            && crate::types::program::is_record_valued(name)
            && let Some(shape) = crate::types::program::record(name)
        {
            return self.construct_record(name, &shape, arguments, span);
        }
        let forms = self.constructor_signatures(name);
        let first = match arguments.first() {
            Some(argument) if argument.name.is_none() => Some(self.expression(&argument.value)?),
            _ => None,
        };
        let chosen = forms.iter().find(|(_, parameters)| {
            parameters.len() == arguments.len()
                && match (&first, parameters.first()) {
                    (Some(actual), Some(parameter)) => {
                        parameter.has_variable() || actual.is(parameter)
                    }
                    (None, None) => true,
                    _ => false,
                }
        });
        let Some((bound, _)) = chosen else {
            let taken = forms
                .iter()
                .map(|(_, parameters)| {
                    format!(
                        "({})",
                        parameters
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(" or ");
            let given = first.map_or_else(|| "no argument".to_owned(), |ty| ty.to_string());
            return Err(Diagnostic::typing(
                span,
                format!(
                    "`{name}` has no constructor taking {given} with {} argument{}: its forms \
                     take {taken}",
                    arguments.len(),
                    if arguments.len() == 1 { "" } else { "s" }
                ),
            ));
        };
        let bound = bound.clone();
        let ty = self.scope.get(&bound).cloned().unwrap_or(TypeRef::NEVER);
        self.checked
            .module_refs
            .insert(callee.span.clone(), bound.clone());
        self.checked.types.insert(callee.span.clone(), ty.clone());
        self.apply_key(
            Some(&bound),
            &ty,
            callee.span.clone(),
            None,
            arguments,
            span,
        )
    }

    /// A program's constructor passed as a function value: the one form
    /// whose parameters take what the receiving operation passes.
    fn constructor_value(
        &mut self,
        name: &str,
        span: &Range<usize>,
        parameters: &[TypeRef],
    ) -> Result<TypeRef, Diagnostic> {
        let forms = self.constructor_signatures(name);
        let taking: Vec<&(String, Vec<TypeRef>)> = forms
            .iter()
            .filter(|(_, held)| {
                held.len() == parameters.len()
                    && held
                        .iter()
                        .zip(parameters)
                        .all(|(held, passed)| held.has_variable() || passed.is(held))
            })
            .collect();
        let firsts = |candidates: &[&(String, Vec<TypeRef>)]| {
            candidates
                .iter()
                .filter_map(|(_, held)| held.first().map(|first| format!("`{first}`")))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let passed = parameters
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let [(bound, _)] = taking.as_slice() else {
            let all: Vec<&(String, Vec<TypeRef>)> = forms.iter().collect();
            return Err(Diagnostic::typing(
                span.clone(),
                if taking.is_empty() {
                    format!(
                        "`{name}` passed here takes {passed}, and no form of it does: its forms \
                         take {}",
                        firsts(&all)
                    )
                } else {
                    format!(
                        "`{name}` passed here is ambiguous: {passed} fits its forms taking {}; \
                         write the call, `x => {name}(x)`",
                        firsts(&taking)
                    )
                },
            ));
        };
        let bound = bound.clone();
        let ty = self.scope.get(&bound).cloned().unwrap_or(TypeRef::NEVER);
        self.checked.module_refs.insert(span.clone(), bound.clone());
        self.using(&bound, self.effectful.contains_key(&bound));
        let (_, result) = ty.as_function().ok_or_else(|| {
            Diagnostic::typing(span.clone(), format!("`{name}` is not a function"))
        })?;
        let result = result.clone();
        self.checked.types.insert(span.clone(), ty);
        Ok(result)
    }

    /// The signature and body of a declared function, with its type
    /// variables and trait bounds in force.
    fn function_body(&mut self, declaration: &FnDecl, bound: &str) -> Result<(), Diagnostic> {
        let mut parameters = Vec::new();
        let mut names: Vec<String> = Vec::new();
        if declaration.has_self {
            let Some(receiver) = self.receiver.clone() else {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!(
                        "`{}` takes `self`, which only a receiver module declares: open the \
                         module file with `module for T`",
                        declaration.name
                    ),
                ));
            };
            parameters.push(receiver);
            names.push("self".to_owned());
        }
        names.extend(
            declaration
                .parameters
                .iter()
                .map(|parameter| parameter.name.clone()),
        );
        for parameter in &declaration.parameters {
            let ty = crate::declarations::annotation(&parameter.annotation)?;
            parameters.push(if parameter.variadic {
                TypeRef::list(ty)
            } else {
                ty
            });
        }
        let result = crate::declarations::annotation(&declaration.result)?;
        let ty = TypeRef::function(parameters.clone(), result.clone()).ok_or_else(|| {
            Diagnostic::typing(
                declaration.span.clone(),
                "a function takes at most eight parameters",
            )
        })?;
        self.scope.insert(bound.to_owned(), ty.clone());
        if declaration
            .parameters
            .last()
            .is_some_and(|last| last.variadic)
        {
            self.variadic.insert(bound.to_owned(), parameters.len());
        } else {
            self.variadic.remove(bound);
        }
        self.checked.functions.insert(declaration.span.clone(), ty);
        self.parameter_names.insert(bound.to_owned(), names.clone());
        let names = names.into_iter();
        if let Kind::Block(statements) = &declaration.body.kind {
            return self.with_bindings(names.zip(parameters), |checker| {
                checker.iterating(declaration, statements, &result)
            });
        }
        let body = self.with_bindings(names.zip(parameters), |checker| {
            checker.expecting(&declaration.body, Some(&result))
        })?;
        if !body.is(&result) {
            return Err(Diagnostic::typing(
                declaration.body.span.clone(),
                format!(
                    "`{}` is declared to return {result} but its body is {body}",
                    declaration.name
                ),
            ));
        }
        Ok(())
    }

    /// A hole's type: the parameter it fills in the call it stands in.
    fn hole(&mut self, name: &str, call: &Expr) -> Result<TypeRef, Diagnostic> {
        let Kind::Call { callee, arguments } = &call.kind else {
            return Err(Diagnostic::typing(
                call.span.clone(),
                "a hole stands in a call",
            ));
        };
        let position = arguments
            .iter()
            .position(|argument| matches!(&argument.value.kind, Kind::Ident(held) if held == name))
            .unwrap_or_default();
        let untyped = || {
            Diagnostic::typing(
                call.span.clone(),
                "a hole in this call takes its type from where the function is used; \
                 pass it where a function is expected, or write a lambda with typed parameters",
            )
        };
        if !self.names_function(callee) && matches!(callee.kind, Kind::Ident(_)) {
            return Err(untyped());
        }
        let ty = self.expression(callee)?;
        let (parameters, _) = ty.as_function().ok_or_else(untyped)?;
        parameters.get(position).cloned().ok_or_else(|| {
            Diagnostic::typing(
                call.span.clone(),
                format!("this call has more arguments than {ty} takes"),
            )
        })
    }

    /// `name(p, ..) := body`: a function whose types the checker writes. The
    /// input of a fragment takes the most general type the body accepts, an
    /// unannotated parameter the type of the first step argument it is
    /// passed to, and the result whatever the body returns.
    fn alias(&mut self, alias: &crate::ast::AliasDecl) -> Result<(), Diagnostic> {
        constructor_name(&self.declarations, &alias.name, None, &alias.span)?;
        use crate::types::builtin::{PLACEHOLDERS, type_variable};
        self.lint(crate::naming::Kind::Function, &alias.name, &alias.span);
        if builtins::exists(&alias.name) {
            self.checked.warnings.push(crate::warnings::Warning::new(
                alias.span.clone(),
                format!("`{}` shadows the step of the same name", alias.name),
            ));
        }
        let implicit = alias
            .parameters
            .first()
            .is_some_and(|input| input.name == crate::ast::ALIAS_INPUT);
        for parameter in alias.parameters.iter().skip(usize::from(implicit)) {
            self.lint(
                crate::naming::Kind::Parameter,
                &parameter.name,
                &parameter.span,
            );
        }
        if alias.parameters.len() > 4 {
            return Err(Diagnostic::typing(
                alias.span.clone(),
                "a function takes at most eight parameters",
            ));
        }
        let candidates: Vec<TypeRef> = if implicit {
            let elements = [
                type_variable("T", None),
                type_variable("T", Some(&TypeRef::ORDERABLE)),
                Some(TypeRef::INT),
                Some(TypeRef::FLOAT),
                Some(TypeRef::STR),
            ];
            elements
                .into_iter()
                .flatten()
                .flat_map(|element| {
                    [
                        collections::ITERABLE,
                        collections::STREAM,
                        collections::COLLECTION,
                        collections::SEQ,
                    ]
                    .map(|shape| shape.apply([element.clone()]))
                })
                .collect()
        } else {
            let input = alias.parameters.first().ok_or_else(|| {
                Diagnostic::typing(
                    alias.span.clone(),
                    "an alias whose body is not a pipeline fragment names its input as its \
                     first parameter",
                )
            })?;
            let annotation = input.annotation.as_ref().ok_or_else(|| {
                Diagnostic::typing(
                    input.span.clone(),
                    format!(
                        "the input `{}` of an expression body is annotated: `{}: Type`",
                        input.name, input.name
                    ),
                )
            })?;
            vec![crate::declarations::annotation(annotation)?]
        };
        let mut first_failure = None;
        for input in candidates {
            let warnings = self.checked.warnings.len();
            self.placeholders.clear();
            let mut held = Vec::new();
            let mut slots = PLACEHOLDERS.iter();
            for (index, parameter) in alias.parameters.iter().enumerate() {
                let ty = if index == 0 {
                    input.clone()
                } else if let Some(annotation) = &parameter.annotation {
                    crate::declarations::annotation(annotation)?
                } else {
                    let slot = slots.next().ok_or_else(|| {
                        Diagnostic::typing(alias.span.clone(), "too many parameters to infer")
                    })?;
                    crate::types::TypeConstructor(slot).apply([])
                };
                held.push((parameter.name.clone(), ty));
            }
            let body = self.with_bindings(held.clone(), |checker| checker.expression(&alias.body));
            let body = match body {
                Ok(body) => body,
                Err(failure) => {
                    self.checked.warnings.truncate(warnings);
                    // The candidate that got furthest into the body names
                    // the expression whose type is unknown.
                    let further = first_failure
                        .as_ref()
                        .is_none_or(|held: &Diagnostic| failure.span().start > held.span().start);
                    if further {
                        first_failure = Some(failure);
                    }
                    continue;
                }
            };
            let fixed = |ty: &TypeRef| substituted_placeholders(ty, &self.placeholders);
            let mut parameters = Vec::new();
            for (name, ty) in &held {
                let ty = fixed(ty);
                if ty.has_placeholder() {
                    return Err(Diagnostic::typing(
                        alias.span.clone(),
                        format!(
                            "the body of `{}` does not fix the type of `{name}`; annotate it, \
                             `{name}: Type`, or write `fn`",
                            alias.name
                        ),
                    ));
                }
                parameters.push(ty);
            }
            let result = fixed(&body);
            let ty = TypeRef::function(parameters.clone(), result).ok_or_else(|| {
                Diagnostic::typing(
                    alias.span.clone(),
                    "a function takes at most eight parameters",
                )
            })?;
            let variables: Vec<(String, TypeRef, Option<TypeRef>)> = [false, true]
                .into_iter()
                .filter_map(|orderable| {
                    let bound = orderable.then_some(TypeRef::ORDERABLE);
                    let variable = type_variable("T", bound.as_ref())?;
                    parameters
                        .iter()
                        .any(|parameter| parameter.mentions(&variable))
                        .then(|| ("T".to_owned(), variable, bound))
                })
                .collect();
            if variables.is_empty() {
                self.generic_functions.remove(&alias.name);
            } else {
                self.generic_functions.insert(alias.name.clone(), variables);
            }
            self.parameter_names.insert(
                alias.name.clone(),
                held.iter()
                    .map(|(name, _)| name.trim_start_matches('%').to_owned())
                    .collect(),
            );
            self.scope.insert(alias.name.clone(), ty.clone());
            self.checked
                .functions
                .insert(alias.span.clone(), ty.clone());
            self.checked.signatures.push((alias.name.clone(), ty));
            return Ok(());
        }
        let failure = first_failure
            .unwrap_or_else(|| Diagnostic::typing(alias.span.clone(), "the body did not check"));
        Err(Diagnostic::typing(
            failure.span().clone(),
            format!(
                "{}; annotate the input of `{}`, or write `fn`",
                failure.message(),
                alias.name
            ),
        ))
    }

    /// The block body of an iterating function: what it yields must be the
    /// element its declared `Seq<T>` or `Stream<T>` promises.
    fn iterating(
        &mut self,
        declaration: &FnDecl,
        statements: &[crate::ast::BlockStmt],
        result: &TypeRef,
    ) -> Result<(), Diagnostic> {
        let element = match result.args.as_slice() {
            [element] if matches!(result.constructor, collections::SEQ | collections::STREAM) => {
                element.clone()
            }
            _ => {
                return Err(Diagnostic::typing(
                    declaration.result.span.clone(),
                    format!(
                        "`{}` has a block body, which yields, so it returns Seq<T> or \
                         Stream<T>, not {result}",
                        declaration.name
                    ),
                ));
            }
        };
        let yields = self.block(&declaration.name, statements, &element)?;
        if yields == 0 {
            return Err(Diagnostic::typing(
                declaration.body.span.clone(),
                format!(
                    "`{}` has a block body and never yields: a block body produces its \
                     values with `yield`",
                    declaration.name
                ),
            ));
        }
        self.checked
            .types
            .insert(declaration.body.span.clone(), result.clone());
        Ok(())
    }

    /// Check a block's statements in order and count its `yield`s. A binding
    /// is visible to the statements after it and not outside the block.
    fn block(
        &mut self,
        function: &str,
        statements: &[crate::ast::BlockStmt],
        element: &TypeRef,
    ) -> Result<usize, Diagnostic> {
        use crate::ast::BlockStmt;
        let outer = self.scope.clone();
        let mut yields = 0;
        let mut checked = || -> Result<(), Diagnostic> {
            for statement in statements {
                match statement {
                    BlockStmt::Bind { name, value } => {
                        self.lint(crate::naming::Kind::Binding, name, &value.span);
                        let ty = self.expression(value)?;
                        self.scope.insert(name.clone(), ty);
                    }
                    BlockStmt::Yield(value) => {
                        let ty = self.expecting(value, Some(element))?;
                        if !ty.is(element) {
                            return Err(Diagnostic::typing(
                                value.span.clone(),
                                format!("`{function}` yields {element}, and this is {ty}"),
                            ));
                        }
                        yields += 1;
                    }
                    BlockStmt::For {
                        name,
                        iterable,
                        body,
                    } => {
                        let ty = self.expression(iterable)?;
                        let Some(item) = ty.present().iterated() else {
                            return Err(Diagnostic::typing(
                                iterable.span.clone(),
                                format!("`for` traverses a collection or a stream, not {ty}"),
                            ));
                        };
                        self.lint(crate::naming::Kind::Binding, name, &iterable.span);
                        let inner = self.scope.insert(name.clone(), item);
                        yields += self.block(function, body, element)?;
                        match inner {
                            Some(held) => self.scope.insert(name.clone(), held),
                            None => self.scope.remove(name),
                        };
                    }
                }
            }
            Ok(())
        };
        let outcome = checked();
        self.scope = outer;
        outcome.map(|()| yields)
    }

    /// Apply a function value to arguments, the first of which may be a
    /// piped input. A call by the name of a variadic function collects its
    /// trailing arguments into the last parameter.
    fn application(
        &mut self,
        callee: &Expr,
        input: Option<&TypeRef>,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        let ty = self.expression(callee)?;
        let key = self.callee_key(callee);
        self.apply_key(
            key.as_deref(),
            &ty,
            callee.span.clone(),
            input,
            arguments,
            span,
        )
    }

    /// The name a callee is known by, when it has one: a bound name, or a
    /// module's function as `module.name`.
    fn callee_key(&self, callee: &Expr) -> Option<String> {
        match &callee.kind {
            Kind::Ident(name) => Some(name.clone()),
            Kind::Field(receiver, field) => match &receiver.kind {
                Kind::Ident(module) if self.is_module(module) => Some(format!("{module}.{field}")),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether a name is a workspace module imported here and not a binding.
    fn is_module(&self, name: &str) -> bool {
        !self.scope.contains_key(name) && self.modules.iter().any(|(held, _)| held == name)
    }

    /// Apply a function of a known type, known by `key` when it is a named
    /// function whose generics, variadic parameter and parameter names are
    /// on record.
    fn apply_key(
        &mut self,
        key: Option<&str>,
        ty: &TypeRef,
        callee_span: Range<usize>,
        input: Option<&TypeRef>,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        if let Some(name) = key {
            let effectful = self.effectful.contains_key(name);
            self.using(name, effectful);
        }
        let Some((parameters, result)) = ty.as_function() else {
            return Err(Diagnostic::typing(
                callee_span,
                format!("only a function or a pipeline step can be called, and this is {ty}"),
            ));
        };
        let (mut parameters, mut result) = (parameters.to_vec(), result.clone());
        let generic = key.and_then(|name| self.generic_functions.get(name).cloned());
        if let Some(argument) = arguments.iter().find(|argument| argument.name.is_some()) {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "a function value takes its arguments by position",
            ));
        }
        let variadic = key.and_then(|name| self.variadic.get(name).copied());
        let mut supplied: Vec<(TypeRef, Range<usize>)> = Vec::new();
        if let Some(input) = input {
            supplied.push((input.clone(), span.clone()));
        }
        for (index, argument) in arguments.iter().enumerate() {
            let position = index + usize::from(input.is_some());
            let expected = match variadic {
                Some(count) if position + 1 >= count => parameters[count - 1].element(),
                _ => parameters.get(position).cloned(),
            }
            // A parameter that holds a type variable is filled by the argument,
            // not the other way round.
            .filter(|expected| !expected.has_variable());
            let actual = self.expecting(&argument.value, expected.as_ref())?;
            supplied.push((actual, argument.value.span.clone()));
        }
        let pack = match variadic {
            Some(count) => {
                let fixed = count - 1;
                if supplied.len() < fixed {
                    return Err(Diagnostic::typing(
                        span,
                        format!("this function takes at least {fixed} arguments"),
                    ));
                }
                let mut element = parameters[fixed].element().unwrap_or(TypeRef::DATA);
                // `...T` binds one type for every trailing argument: the
                // fixed arguments and the trailing ones fill it as repeated
                // parameters do, a later argument widening it only to a
                // declared ancestor of what an earlier one bound.
                let generic_element = element.has_variable();
                if generic_element {
                    let mut bound = HashMap::new();
                    for ((actual, _), parameter) in supplied[..fixed].iter().zip(&parameters) {
                        instantiate(parameter, actual, &mut bound);
                    }
                    for (actual, _) in &supplied[fixed..] {
                        instantiate(&element, actual, &mut bound);
                    }
                    element = substituted(&element, &bound);
                    if element.has_variable() {
                        // No argument fills it: the pack is empty.
                        element = TypeRef::NEVER;
                    }
                }
                for (actual, at) in &supplied[fixed..] {
                    if !actual.is(&element) {
                        return Err(Diagnostic::typing(
                            at.clone(),
                            if generic_element {
                                format!(
                                    "the trailing arguments fill one type, here {element}, \
                                     and this one is {actual}"
                                )
                            } else {
                                format!("a trailing argument here is {element}, not {actual}")
                            },
                        ));
                    }
                }
                supplied.truncate(fixed);
                Some((fixed, element))
            }
            None => {
                if supplied.len() != parameters.len() {
                    return Err(Diagnostic::typing(
                        span,
                        format!(
                            "this function takes {} arguments, not {}",
                            parameters.len(),
                            supplied.len()
                        ),
                    ));
                }
                None
            }
        };
        if let Some(variables) = &generic {
            let callee_name = key.unwrap_or("");
            let mut bound = HashMap::new();
            for ((actual, _), parameter) in supplied.iter().zip(&parameters) {
                instantiate(parameter, actual, &mut bound);
            }
            if let Some((fixed, element)) = &pack
                && let Some(parameter) = parameters.get(*fixed)
            {
                instantiate(parameter, &TypeRef::list(element.clone()), &mut bound);
            }
            if let Some(bounds) = self.trait_bounded.get(callee_name).cloned() {
                self.resolve_bounds(callee_name, &bounds, &mut bound, &span)?;
            }
            for (name, variable, limit) in variables {
                let filled = bound
                    .get(&variable.constructor)
                    .cloned()
                    .unwrap_or(TypeRef::NEVER);
                if let Some(limit) = limit
                    && !filled.is(limit)
                {
                    return Err(Diagnostic::typing(
                        span,
                        format!(
                            "`{callee_name}` needs {name} : {limit}, and {name} is {filled} here"
                        ),
                    ));
                }
            }
            parameters = parameters
                .iter()
                .map(|parameter| substituted(parameter, &bound))
                .collect();
            result = substituted(&result, &bound);
        }
        let names = key.and_then(|name| self.parameter_names.get(name).cloned());
        for (index, ((actual, at), parameter)) in supplied.iter().zip(&parameters).enumerate() {
            if !actual.is(parameter) {
                let which = names
                    .as_ref()
                    .and_then(|names| names.get(index))
                    .map_or_else(
                        || format!("parameter {}", index + 1),
                        |name| format!("parameter `{name}`"),
                    );
                return Err(Diagnostic::typing(
                    at.clone(),
                    format!("this argument is {actual}, and {which} takes {parameter}"),
                ));
            }
        }
        self.checked.applications.insert(span, pack);
        Ok(result)
    }

    /// `receiver.f(args)`: a call with the receiver as its first argument,
    /// or `module.f(args)` where the receiver names a visible module and no
    /// binding. A receiver call resolves, in order, a combiner or `compare`
    /// over the receiver, and a step of a visible module whose input the
    /// receiver's type fits.
    fn dot_call(
        &mut self,
        callee: &Expr,
        receiver: &Expr,
        name: &str,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        let positional = arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .count();
        // `geo.area(c)`: a function of a workspace module, applied.
        if let Kind::Ident(module) = &receiver.kind
            && self.is_module(module)
        {
            return self.application(callee, None, arguments, span);
        }
        // `string.trim(s)`: the module qualifies the function, and the first
        // argument is its input.
        if let Kind::Ident(module) = &receiver.kind
            && !self.scope.contains_key(module)
            && self.resolution.has(module)
        {
            let step = self.resolution.qualified(module, name, &span)?;
            let Some((input, rest)) = arguments.split_first() else {
                return Err(Diagnostic::typing(
                    span,
                    format!("`{module}.{name}` needs its input as the first argument"),
                ));
            };
            if input.name.is_some() {
                return Err(Diagnostic::typing(
                    input.value.span.clone(),
                    "the input must be positional",
                ));
            }
            let input_type = self.expression(&input.value)?;
            self.checked.steps.insert(span.clone(), step);
            self.using(step.name, step.purity() == Purity::Effectful);
            return self.step(step, &input_type, Some(&input.value), rest, span);
        }
        let receiver_type = self.expression(receiver)?;
        // A function of a receiver module whose pattern the receiver's type
        // fits: `"hi".shout()` once `mighty.text` is imported.
        if let Some((key, ty)) = self.receiver_function(&receiver_type, name, &span)? {
            self.checked
                .module_calls
                .insert(span.clone(), (key.clone(), ty.clone()));
            return self.apply_key(
                Some(&key),
                &ty,
                receiver.span.clone(),
                Some(&receiver_type),
                arguments,
                span,
            );
        }
        // A trait's function, implemented for the receiver's type or
        // promised by a bound on it. Implementations at several arguments
        // are told apart by the types of the call's own.
        // A step of the same name at another argument count is the other
        // form: `s.slice(range)` is `Slice`'s, `s.slice(start, end)` the
        // String module's.
        let trait_route =
            self.method_for(&receiver_type, name, &span)?
                .filter(|(_, signatures)| {
                    signatures[0].parameters.len() == arguments.len() || !builtins::exists(name)
                });
        if let Some((trait_name, signatures)) = trait_route {
            let count = signatures[0].parameters.len();
            if arguments.len() != count {
                return Err(Diagnostic::typing(
                    span,
                    format!(
                        "`{name}` of `{trait_name}` takes {count} argument{} after the receiver, not {}",
                        if count == 1 { "" } else { "s" },
                        arguments.len()
                    ),
                ));
            }
            if let Some(argument) = arguments.iter().find(|argument| argument.name.is_some()) {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    "a trait's function takes its arguments by position",
                ));
            }
            let signature = if let [signature] = signatures.as_slice() {
                for (argument, parameter) in arguments.iter().zip(&signature.parameters) {
                    let actual = self.expecting(&argument.value, Some(parameter))?;
                    if !actual.is(parameter) {
                        return Err(Diagnostic::typing(
                            argument.value.span.clone(),
                            format!("this argument is {actual}, and `{name}` takes {parameter}"),
                        ));
                    }
                }
                signature.clone()
            } else {
                let actuals = arguments
                    .iter()
                    .map(|argument| self.expression(&argument.value))
                    .collect::<Result<Vec<_>, _>>()?;
                let chosen: Vec<&MethodSig> = signatures
                    .iter()
                    .filter(|signature| {
                        actuals
                            .iter()
                            .zip(&signature.parameters)
                            .all(|(actual, parameter)| actual.is(parameter))
                    })
                    .collect();
                let [chosen] = chosen.as_slice() else {
                    return Err(Diagnostic::typing(
                        span,
                        format!(
                            "`{name}` of `{trait_name}` is implemented for {receiver_type} at \
                             several arguments, and this call does not choose between them"
                        ),
                    ));
                };
                (*chosen).clone()
            };
            self.checked
                .method_calls
                .insert(span, (trait_name, name.to_owned()));
            return Ok(signature.result);
        }
        if let Some(combiner) = crate::extensions::combiner(name).map(|held| held.name)
            && positional == 1
            && arguments.len() == 1
            && !self.scope.contains_key(name)
        {
            let right = self.expression(&arguments[0].value)?;
            let ty = combined(combiner, &receiver_type, &right)
                .map_err(|message| Diagnostic::typing(span.clone(), message))?;
            self.checked.combiners.insert(span.clone(), combiner);
            self.checked.receiver_calls.insert(span);
            return Ok(ty);
        }
        if name == "compare" && arguments.len() == 1 && arguments[0].name.is_none() {
            let right = self.expression(&arguments[0].value)?;
            if receiver_type != right || !receiver_type.is_orderable() {
                return Err(Diagnostic::typing(
                    span,
                    "compare requires two values of the same Orderable type",
                ));
            }
            self.checked.receiver_calls.insert(span);
            return Ok(TypeRef::ORDERING);
        }
        if builtins::exists(name) {
            self.not_own_function(name, &receiver_type, &span)?;
            let step =
                self.resolution
                    .receiver_step_for(name, &receiver_type, positional, &span)?;
            self.checked.steps.insert(span.clone(), step);
            self.checked.receiver_calls.insert(span.clone());
            self.using(step.name, step.purity() == Purity::Effectful);
            return self.step(step, &receiver_type, Some(receiver), arguments, span);
        }
        let hint = builtins::hint(name);
        Err(Diagnostic::name(
            span,
            format!("{receiver_type} has no function `{name}`{hint}"),
        ))
    }

    /// `fn name(p: T) : R` without a body: the signature of a function an
    /// extension implements. It is checked and recorded, and binds nothing:
    /// the implementation is reached through the module's registry entry.
    fn external(&mut self, declaration: &ExternalDecl) -> Result<(), Diagnostic> {
        constructor_name(
            &self.declarations,
            &declaration.name,
            Some(&declaration.result),
            &declaration.span,
        )?;
        if !crate::constructors::is_constructor_name(&declaration.name) {
            self.lint(
                crate::naming::Kind::Function,
                &declaration.name,
                &declaration.span,
            );
        }
        let mut variables = Vec::new();
        // A constructor parameter, `F<_>`, is written applied, `F<T>`; as a
        // type it is the variable it binds.
        let constructors: Vec<&str> = declaration
            .bounds
            .iter()
            .filter(|(_, bound)| bound.is_constructor())
            .map(|(name, _)| name.as_str())
            .collect();
        let erase = |written: &crate::ast::TypeAnn| erase_constructors(written, &constructors);
        for name in &declaration.generics {
            // A bound written with arguments, `S : Seq<T>` or
            // `F<_> : Iterable<_>`, is a shape the call's input meets, not a
            // family of variables.
            let bound = declaration
                .bounds
                .iter()
                .find(|(bounded, _)| bounded == name)
                .filter(|(_, bound)| !self.traits.contains_key(&bound.name))
                .filter(|(_, bound)| bound.arguments.is_empty())
                .map(|(_, bound)| crate::declarations::annotation(bound))
                .transpose()?;
            if let Some(variable) = crate::types::builtin::type_variable(name, bound.as_ref()) {
                variables.push((name.clone(), variable));
            }
        }
        // `...Any`: a value of any type in each position, held as a variable
        // the declaration does not otherwise name.
        if let Some(variable) = ('A'..='Z')
            .map(String::from)
            .find(|letter| !declaration.generics.contains(letter))
            .and_then(|letter| crate::types::builtin::type_variable(&letter, None))
        {
            variables.push(("Any".to_owned(), variable));
        }
        let _variables = crate::declarations::TypeVariables::enter(&variables);
        let mut parameters = Vec::new();
        // A written receiver, `self: Collection<T>`, is the type it names,
        // in a receiver module or outside one.
        if let Some(written) = declaration
            .receiver
            .as_ref()
            .filter(|_| declaration.has_self)
        {
            parameters.push(crate::declarations::annotation(&erase(written))?);
        } else if declaration.has_self {
            let Some(receiver) = self.receiver.clone() else {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!(
                        "`{}` takes `self`, which only a receiver module declares: open the \
                         module file with `module for T`",
                        declaration.name
                    ),
                ));
            };
            parameters.push(receiver);
        }
        for parameter in &declaration.parameters {
            self.lint(
                crate::naming::Kind::Parameter,
                &parameter.name,
                &parameter.span,
            );
            let ty = crate::declarations::annotation(&erase(&parameter.annotation))?;
            parameters.push(if parameter.variadic {
                TypeRef::list(ty)
            } else {
                ty
            });
        }
        let result = crate::declarations::annotation(&erase(&declaration.result))?;
        let ty = TypeRef::function(parameters, result).ok_or_else(|| {
            Diagnostic::typing(
                declaration.span.clone(),
                "a function takes at most eight parameters",
            )
        })?;
        self.externals.insert(declaration.name.clone(), ty);
        Ok(())
    }

    /// `import a.b [as c]` of a workspace module: load it once, and bind its
    /// functions, traits and implementations under the local name.
    fn import_module(
        &mut self,
        name: &str,
        local: &str,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        if self.resolution.names_module(local) {
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{local}` already names a standard module here; import `{name}` under \
                     another name with `as`"
                ),
            ));
        }
        let module = self.load_module(name, span)?;
        for function in &module.functions {
            let key = format!("{local}.{}", function.name);
            self.scope.insert(key.clone(), function.ty.clone());
            if let Some(count) = function.variadic {
                self.variadic.insert(key.clone(), count);
            }
            self.parameter_names
                .insert(key.clone(), function.parameter_names.clone());
            if let Some(generics) = &function.generics {
                self.generic_functions.insert(key.clone(), generics.clone());
            }
            if let Some(bounds) = &function.trait_bounds {
                self.trait_bounded.insert(key.clone(), bounds.clone());
            }
            if let Some(uses) = &function.effectful {
                self.effectful.insert(key, uses.clone());
            }
        }
        for (binding, ty) in &module.bindings {
            self.scope.insert(format!("{local}.{binding}"), ty.clone());
        }
        for private in &module.private_types {
            crate::declarations::hide(private, &module.name);
        }
        for (trait_name, info) in &module.traits {
            self.traits
                .entry(trait_name.clone())
                .or_insert_with(|| info.clone());
        }
        for held in &module.impls {
            let known = self.impls.iter().find(|known| known.overlaps(held));
            if let Some(known) = known {
                // The one implementation reached through two imports is
                // still one; two implementations are an incoherence.
                if known.origin == held.origin && known.span == held.span {
                    continue;
                }
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!(
                        "`{}` is implemented for {} in {} and in {}: a trait has one \
                         implementation per type, and import order does not choose",
                        trait_written(&held.trait_name, &held.arguments),
                        held.target,
                        known.whence(),
                        held.whence()
                    ),
                ));
            }
            {
                self.impls.push(held.clone());
                if held.trait_name == "Ord" {
                    crate::types::program::adopt_parent(
                        held.target.constructor,
                        TypeRef::ORDERABLE.constructor,
                    )
                    .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
                }
            }
        }
        self.checked.modules.push(crate::execution::LoadedModule {
            local: local.to_owned(),
            program: std::sync::Arc::clone(&module.program),
        });
        self.modules.push((local.to_owned(), module));
        Ok(())
    }

    /// Find, parse and check a workspace module, once per program.
    fn load_module(
        &mut self,
        name: &str,
        span: &Range<usize>,
    ) -> Result<std::sync::Arc<crate::modules::WorkspaceModule>, Diagnostic> {
        if let Some(module) = self
            .module_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(name)
        {
            return Ok(std::sync::Arc::clone(module));
        }
        let roots = &self.vault.extensions.module_roots;
        let located = crate::modules::locate(roots, name)
            .map_err(|message| Diagnostic::name(span.clone(), message))?;
        let Some(path) = located else {
            let hint = if roots.is_empty() {
                "; no module root is configured: pass `--modules <dir>` or set `[modules] \
                 roots` in hql.toml"
                    .to_owned()
            } else {
                format!(
                    " under {}",
                    roots
                        .iter()
                        .map(|root| root.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            return Err(Diagnostic::name(
                span.clone(),
                format!("no module named `{name}`{hint}"),
            ));
        };
        {
            let loading = self
                .loading
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(position) = loading.iter().position(|held| held == name) {
                let cycle: Vec<&str> = loading[position..]
                    .iter()
                    .map(String::as_str)
                    .chain(std::iter::once(name))
                    .collect();
                return Err(Diagnostic::name(
                    span.clone(),
                    format!(
                        "importing `{name}` is circular: {}",
                        cycle.join(" imports ")
                    ),
                ));
            }
        }
        let source = std::fs::read_to_string(&path).map_err(|error| {
            Diagnostic::name(
                span.clone(),
                format!("module `{name}` at {}: {error}", path.display()),
            )
        })?;
        let inside = |diagnostic: Diagnostic| {
            Diagnostic::name(
                span.clone(),
                format!(
                    "in module `{name}` ({}), at byte {}: {}",
                    path.display(),
                    diagnostic.span().start,
                    diagnostic.message()
                ),
            )
        };
        let program = crate::parser::parse(&source).map_err(inside)?;
        self.loading
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(name.to_owned());
        let mut checker = Checker {
            unit: name.to_owned(),
            module_cache: std::sync::Arc::clone(&self.module_cache),
            loading: std::sync::Arc::clone(&self.loading),
            module_file: true,
            // A module is a closure: it sees no host binding and none of the
            // program's, and only the standard prelude among implicit imports.
            ..Checker::new(
                self.vault,
                HashMap::new(),
                Resolution::bare(self.vault.extensions.prelude),
            )
        };
        let checked = {
            // The unit's own imports decide which private types it may not
            // name; the importer's do not reach it.
            let _hidden = crate::declarations::HiddenTypes::enter();
            checker
                .program(&program)
                .and_then(|typed| public_signatures(&program).map(|()| typed))
        };
        self.loading
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop();
        let typed = checked.map_err(inside)?;
        // What checking the module had to say is said where it is imported,
        // the one place the importer's report can point at.
        for warning in std::mem::take(&mut checker.checked.warnings) {
            CheckCx::warn(
                self,
                crate::warnings::Warning::new(
                    span.clone(),
                    format!("in module `{name}`: {}", warning.message),
                ),
            );
        }
        let mut functions = Vec::new();
        for statement in &program.statements {
            // A private declaration is never exported: no importer reaches
            // it, by name, as a receiver call or as a step.
            let Stmt::Fn(declaration) = statement else {
                continue;
            };
            if declaration.private.is_some() {
                continue;
            }
            let Some(ty) = checker.scope.get(&declaration.name).cloned() else {
                continue;
            };
            let mut generics = checker.generic_functions.get(&declaration.name).cloned();
            // The receiver pattern's variables are the function's own.
            if declaration.has_self
                && let Some(receiver) = &checker.receiver
                && receiver.has_variable()
            {
                let held = generics.get_or_insert_with(Vec::new);
                for (written, variable) in pattern_variables(receiver) {
                    if !held.iter().any(|(name, _, _)| *name == written) {
                        held.push((written, variable, None));
                    }
                }
            }
            functions.push(crate::modules::ModuleFunction {
                name: declaration.name.clone(),
                ty,
                has_self: declaration.has_self,
                variadic: checker.variadic.get(&declaration.name).copied(),
                parameter_names: checker
                    .parameter_names
                    .get(&declaration.name)
                    .cloned()
                    .unwrap_or_default(),
                generics,
                trait_bounds: checker.trait_bounded.get(&declaration.name).cloned(),
                effectful: checker.effectful.get(&declaration.name).cloned(),
            });
        }
        let private: std::collections::HashSet<&str> = program
            .statements
            .iter()
            .filter(|statement| statement.private().is_some())
            .filter_map(Stmt::declared_name)
            .collect();
        let bindings = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Stmt::Bind {
                    name,
                    private: None,
                    ..
                } => Some((name.clone(), checker.scope.get(name)?.clone())),
                _ => None,
            })
            .collect();
        let private_types = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Stmt::Type(declaration) if declaration.private.is_some() => {
                    Some(declaration.name.clone())
                }
                _ => None,
            })
            .collect();
        let core = core_traits();
        let traits = checker
            .traits
            .iter()
            .filter(|(name, _)| !core.contains_key(*name) && !private.contains(name.as_str()))
            .map(|(name, info)| (name.clone(), info.clone()))
            .collect();
        let module = std::sync::Arc::new(crate::modules::WorkspaceModule {
            name: name.to_owned(),
            receiver: checker.receiver.clone(),
            functions,
            bindings,
            private_types,
            program: std::sync::Arc::new(typed),
            traits,
            impls: checker.impls.clone(),
        });
        self.module_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(name.to_owned(), std::sync::Arc::clone(&module));
        Ok(module)
    }

    /// Refuse a use that would reach an import's `name` while this unit
    /// declares its own further down: the unit's declaration wins wherever
    /// it is written, and a function is used after it is declared.
    fn not_declared_below(&self, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
        if self.declared_below.contains(name) {
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` is declared by this unit below this use; the unit's own \
                     declaration wins over an import's, so declare `{name}` before it is used, \
                     or qualify the import's"
                ),
            ));
        }
        Ok(())
    }

    /// Refuse a receiver call that would reach an import's `name` while this
    /// unit declares its own `name` without `self`: the unit's declaration
    /// wins in every spelling, and it is no receiver.
    fn not_own_function(
        &self,
        name: &str,
        receiver: &TypeRef,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        self.not_declared_below(name, span)?;
        if self.own_functions.contains(name) {
            return Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` is this unit's own function, which takes no `self`, so \
                     `value.{name}(…)` does not reach it and no import's `{name}` over \
                     {receiver} is reached in its place; write `{name}(value, …)`, or qualify \
                     the import's"
                ),
            ));
        }
        Ok(())
    }

    /// The function a receiver module provides for a receiver of a type: the
    /// unit's own when it declares one, and otherwise the imported modules
    /// whose pattern the type fits, among them the one declaring `name` with
    /// `self`. Two such imports, or one beside a visible standard module's
    /// receiver of the name, are an ambiguity that only qualifying resolves.
    fn receiver_function(
        &self,
        receiver: &TypeRef,
        name: &str,
        span: &Range<usize>,
    ) -> Result<Option<(String, TypeRef)>, Diagnostic> {
        if let Some(pattern) = &self.receiver {
            let mut bound = HashMap::new();
            instantiate(pattern, receiver, &mut bound);
            if receiver.is(&substituted(pattern, &bound))
                && let Some((own, ty)) = self.own_receivers.iter().find(|(held, _)| held == name)
            {
                // The unit owns its names: its declaration wins over every
                // import's, whichever was written first.
                return Ok(Some((own.clone(), ty.clone())));
            }
        }
        let mut found: Vec<(String, TypeRef)> = Vec::new();
        for (local, module) in &self.modules {
            let Some(pattern) = &module.receiver else {
                continue;
            };
            let mut bound = HashMap::new();
            instantiate(pattern, receiver, &mut bound);
            if !receiver.is(&substituted(pattern, &bound)) {
                continue;
            }
            if let Some(function) = module
                .functions
                .iter()
                .find(|function| function.has_self && function.name == name)
            {
                found.push((format!("{local}.{name}"), function.ty.clone()));
            }
        }
        if found.is_empty() {
            return Ok(None);
        }
        self.not_own_function(name, receiver, span)?;
        let mut providers: Vec<String> = found.iter().map(|(key, _)| key.clone()).collect();
        providers.extend(
            self.resolution
                .receiver_providers(name, receiver)
                .into_iter()
                .map(|module| format!("{module}.{name}")),
        );
        match providers.len() {
            1 => Ok(found.pop()),
            _ => Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` over {receiver} is declared by {}, so the call is ambiguous; \
                     qualify it",
                    providers
                        .iter()
                        .map(|key| format!("`{key}`"))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            )),
        }
    }

    /// The type parameters a trait or an implementation declares: each as
    /// the type variable it stands for, with the type its arguments meet,
    /// and the variables to enter while its types are read.
    fn declared_parameters(
        &mut self,
        generics: &[String],
        bounds: &[(String, crate::ast::TypeAnn)],
        span: &Range<usize>,
    ) -> Result<(Vec<Parameter>, Variables), Diagnostic> {
        let mut parameters = Vec::new();
        let mut variables: Vec<(String, TypeRef)> = Vec::new();
        for name in generics {
            self.lint(crate::naming::Kind::TypeParameter, name, span);
            if variables.iter().any(|(held, _)| held == name) {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!("the type parameter `{name}` is written twice"),
                ));
            }
            let variable = crate::types::builtin::type_variable(name, None).ok_or_else(|| {
                Diagnostic::typing(
                    span.clone(),
                    format!("a type parameter is one capital letter, such as `T`, not `{name}`"),
                )
            })?;
            let limit = bounds
                .iter()
                .find(|(bounded, _)| bounded == name)
                .map(|(_, bound)| crate::declarations::annotation(bound))
                .transpose()?;
            parameters.push((variable.clone(), limit));
            variables.push((name.clone(), variable));
        }
        Ok((parameters, variables))
    }

    /// `trait Name { … }`: a contract the program may implement. A core
    /// trait restated by `std/` must agree with the binary's.
    fn declare_trait(&mut self, declaration: &'a TraitDecl) -> Result<(), Diagnostic> {
        self.lint(
            crate::naming::Kind::Type,
            &declaration.name,
            &declaration.span,
        );
        if self.trait_decls.contains_key(&declaration.name) {
            return Err(Diagnostic::name(
                declaration.span.clone(),
                format!("the trait `{}` is declared twice", declaration.name),
            ));
        }
        let (parameters, variables) = self.declared_parameters(
            &declaration.generics,
            &declaration.bounds,
            &declaration.span,
        )?;
        let _variables = crate::declarations::TypeVariables::enter(&variables);
        let mut methods = Vec::new();
        for method in &declaration.methods {
            if !method.has_self {
                return Err(Diagnostic::typing(
                    method.span.clone(),
                    format!(
                        "`{}` takes `self`: a trait's function is called on a value of the type",
                        method.name
                    ),
                ));
            }
            self.lint(crate::naming::Kind::Function, &method.name, &method.span);
            if methods
                .iter()
                .any(|held: &MethodSig| held.name == method.name)
            {
                return Err(Diagnostic::name(
                    method.span.clone(),
                    format!("`{}` declares `{}` twice", declaration.name, method.name),
                ));
            }
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| crate::declarations::annotation(&parameter.annotation))
                .collect::<Result<Vec<_>, _>>()?;
            methods.push(MethodSig {
                name: method.name.clone(),
                parameters,
                result: crate::declarations::annotation(&method.result)?,
            });
        }
        if let Some(core) = self.traits.get(&declaration.name) {
            let same = core.parameters == parameters
                && core.methods.len() == methods.len()
                && core.methods.iter().zip(&methods).all(|(held, written)| {
                    held.name == written.name
                        && held.parameters == written.parameters
                        && held.result == written.result
                });
            if !same {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!(
                        "`{}` is a core trait, and this declaration contradicts it",
                        declaration.name
                    ),
                ));
            }
        }
        self.traits.insert(
            declaration.name.clone(),
            TraitInfo {
                parameters,
                methods,
            },
        );
        self.trait_decls
            .insert(declaration.name.clone(), declaration);
        Ok(())
    }

    /// `impl Name for Type { … }`: every function the trait requires, with
    /// `self` at the type, and the trait's defaults for the rest. The unit
    /// declaring it declares the trait or the type, and there is one
    /// implementation per trait and type wherever it comes from.
    fn implement(&mut self, declaration: &'a ImplDecl) -> Result<(), Diagnostic> {
        let trait_name = declaration.trait_name.as_str();
        let info = self.traits.get(trait_name).cloned().ok_or_else(|| {
            Diagnostic::name(
                declaration.span.clone(),
                format!("no trait named `{trait_name}` is in scope"),
            )
        })?;
        let (_, variables) = self.declared_parameters(
            &declaration.generics,
            &declaration.bounds,
            &declaration.span,
        )?;
        let _variables = crate::declarations::TypeVariables::enter(&variables);
        let parameters: Vec<&'static str> = variables
            .iter()
            .map(|(_, variable)| variable.constructor.0)
            .collect();
        let target = crate::declarations::annotation(&declaration.target)?;
        let arguments = self.trait_arguments(&crate::ast::TypeAnn {
            name: trait_name.to_owned(),
            arguments: declaration.trait_arguments.clone(),
            span: declaration.span.clone(),
        })?;
        for (name, variable) in &variables {
            if !target.mentions(variable)
                && !arguments.iter().any(|argument| argument.mentions(variable))
            {
                return Err(Diagnostic::typing(
                    declaration.span.clone(),
                    format!(
                        "`{name}` of this impl appears in neither the type nor the trait's arguments"
                    ),
                ));
            }
        }
        let this = ImplInfo {
            trait_name: trait_name.to_owned(),
            parameters,
            arguments: arguments.clone(),
            target: target.clone(),
            span: declaration.span.clone(),
            origin: self.unit.clone(),
        };
        let written = this.written();
        if declaration.native {
            // The header is the declaration a native implementation is bound
            // to: the standard library's, read again, or nothing.
            let bound = self.impls.iter().any(|held| {
                held.trait_name == trait_name
                    && held.target == target
                    && held.arguments == arguments
                    && held.origin.starts_with("std/")
            });
            if bound {
                return Ok(());
            }
            return Err(Diagnostic::typing(
                declaration.span.clone(),
                format!(
                    "`impl {written}` has no body, and no extension implements \
                     it: a native implementation is declared by a module an extension provides, \
                     and here the functions are written in braces"
                ),
            ));
        }
        if let Some(previous) = self.impls.iter().find(|held| held.overlaps(&this)) {
            let message = if previous.arguments.is_empty() && previous.target == target {
                format!(
                    "`{trait_name}` is already implemented for {target}, in {}: a trait has \
                     one implementation per type",
                    previous.whence()
                )
            } else {
                format!(
                    "`impl {written}` overlaps `impl {}`, in {}: one type at one set of \
                     arguments has one implementation of a trait",
                    previous.written(),
                    previous.whence()
                )
            };
            return Err(Diagnostic::typing(declaration.span.clone(), message));
        }
        if !self.trait_decls.contains_key(trait_name)
            && !self.declarations.holds(target.constructor.0)
        {
            return Err(Diagnostic::typing(
                declaration.span.clone(),
                format!(
                    "`impl {trait_name} for {target}` belongs with `{trait_name}` or with \
                     {target}, and {} declares neither",
                    if self.unit.is_empty() {
                        "this program".to_owned()
                    } else {
                        format!("module `{}`", self.unit)
                    }
                ),
            ));
        }
        for method in &declaration.methods {
            if !info.methods.iter().any(|held| held.name == method.name) {
                return Err(Diagnostic::name(
                    method.span.clone(),
                    format!("`{trait_name}` has no function `{}`", method.name),
                ));
            }
            if !method.has_self {
                return Err(Diagnostic::typing(
                    method.span.clone(),
                    format!("`{}` takes `self`", method.name),
                ));
            }
            if method.body.is_none() {
                return Err(Diagnostic::typing(
                    method.span.clone(),
                    format!("`{}` has a body in an implementation", method.name),
                ));
            }
        }
        let _self_type = crate::declarations::SelfType::enter(target.clone());
        let mut lowered = Vec::new();
        for signature in &info.methods {
            let MethodSig {
                parameters, result, ..
            } = signature_in_place(signature, &target, &info, &arguments);
            let own = declaration
                .methods
                .iter()
                .find(|method| method.name == signature.name);
            let (method, default_of) = match own {
                Some(method) => (method, None),
                None => {
                    let inherited = self
                        .trait_decls
                        .get(trait_name)
                        .and_then(|held| {
                            held.methods
                                .iter()
                                .find(|method| method.name == signature.name)
                        })
                        .filter(|method| method.body.is_some());
                    match inherited {
                        Some(method) => (method, Some(trait_name.to_owned())),
                        None => {
                            return Err(Diagnostic::typing(
                                declaration.span.clone(),
                                format!(
                                    "`impl {trait_name} for {target}` lacks `{}`",
                                    signature.name
                                ),
                            ));
                        }
                    }
                }
            };
            let Some(body) = &method.body else {
                unreachable!("a body was required above");
            };
            if own.is_some() {
                if method.parameters.len() != parameters.len() {
                    return Err(Diagnostic::typing(
                        method.span.clone(),
                        format!(
                            "`{}` of `{trait_name}` takes {} parameter{} after `self`",
                            signature.name,
                            parameters.len(),
                            if parameters.len() == 1 { "" } else { "s" }
                        ),
                    ));
                }
                for (parameter, expected) in method.parameters.iter().zip(&parameters) {
                    let written = crate::declarations::annotation(&parameter.annotation)?;
                    if written != *expected {
                        return Err(Diagnostic::typing(
                            parameter.span.clone(),
                            format!(
                                "`{}` of `{trait_name}` takes {expected} here, and this \
                                 implementation writes {written}",
                                signature.name
                            ),
                        ));
                    }
                }
                let written = crate::declarations::annotation(&method.result)?;
                if written != result {
                    return Err(Diagnostic::typing(
                        method.result.span.clone(),
                        format!(
                            "`{}` of `{trait_name}` returns {result}, and this implementation \
                             writes {written}",
                            signature.name
                        ),
                    ));
                }
            }
            let names: Vec<String> = std::iter::once("self".to_owned())
                .chain(
                    method
                        .parameters
                        .iter()
                        .map(|parameter| parameter.name.clone()),
                )
                .collect();
            let types: Vec<TypeRef> = std::iter::once(target.clone())
                .chain(parameters.iter().cloned())
                .collect();
            // The implementation is visible to its own bodies, so a function
            // may call another of the same trait on `self`.
            self.impls.push(this.clone());
            let body_type = self.with_bindings(
                names.iter().cloned().zip(types.iter().cloned()),
                |checker| checker.expecting(body, Some(&result)),
            );
            self.impls.pop();
            let body_type = body_type?;
            if !body_type.is(&result) {
                return Err(Diagnostic::typing(
                    body.span.clone(),
                    format!(
                        "`{}` of `{trait_name}` returns {result}, and this body is {body_type}",
                        signature.name
                    ),
                ));
            }
            let ty = TypeRef::function(types, result).ok_or_else(|| {
                Diagnostic::typing(
                    method.span.clone(),
                    "a function takes at most eight parameters",
                )
            })?;
            lowered.push(crate::execution::MethodLowering {
                name: signature.name.clone(),
                parameters: names,
                default_of,
                ty,
            });
        }
        self.impls.push(this);
        // A type implementing `Ord` is Orderable: `compare`, `<`, `sort`,
        // `min` and `max` accept it, and its `compare` decides the order.
        if trait_name == "Ord" {
            crate::types::program::adopt_parent(target.constructor, TypeRef::ORDERABLE.constructor)
                .map_err(|error| Diagnostic::typing(declaration.span.clone(), error.to_string()))?;
        }
        self.checked.impls.insert(
            declaration.span.clone(),
            crate::execution::ImplLowering {
                trait_name: trait_name.to_owned(),
                target,
                methods: lowered,
            },
        );
        Ok(())
    }

    /// A trait's arguments as a bound or an implementation writes them,
    /// `Index<T>`: one per parameter of the trait, each within the
    /// parameter's bound.
    fn trait_arguments(&self, written: &crate::ast::TypeAnn) -> Result<Vec<TypeRef>, Diagnostic> {
        let info = self.traits.get(&written.name).ok_or_else(|| {
            Diagnostic::name(
                written.span.clone(),
                format!("no trait named `{}` is in scope", written.name),
            )
        })?;
        if written.arguments.len() != info.parameters.len() {
            let count = info.parameters.len();
            return Err(Diagnostic::typing(
                written.span.clone(),
                format!(
                    "`{}` takes {count} type argument{}, not {}",
                    written.name,
                    if count == 1 { "" } else { "s" },
                    written.arguments.len()
                ),
            ));
        }
        let mut arguments = Vec::new();
        for (argument, (_, limit)) in written.arguments.iter().zip(&info.parameters) {
            let ty = crate::declarations::annotation(argument)?;
            if let Some(limit) = limit
                && !ty.has_variable()
                && !ty.is(limit)
            {
                return Err(Diagnostic::typing(
                    argument.span.clone(),
                    format!("`{}` takes a {limit} here, not {ty}", written.name),
                ));
            }
            arguments.push(ty);
        }
        Ok(arguments)
    }

    /// The ways a type satisfies a trait: each implementation that applies
    /// to it, with the trait's arguments that implementation gives, and the
    /// bounds in force on a type variable. Several that give the same
    /// arguments are one, and one for a narrower type hides one for a wider.
    fn applicable(&self, ty: &TypeRef, trait_name: &str) -> Vec<Applies> {
        // Each with the implementing type it applies as.
        let mut found: Vec<(Applies, TypeRef)> = Vec::new();
        for held in self
            .impls
            .iter()
            .filter(|held| held.trait_name == trait_name)
        {
            if held.target.is_union() && ty.constructor != held.target.constructor {
                continue;
            }
            let mut fixed = HashMap::new();
            if !unify(&held.target, ty, &held.parameters, &mut fixed) {
                continue;
            }
            let fixed: HashMap<crate::types::TypeConstructor, TypeRef> = fixed
                .into_iter()
                .map(|(parameter, ty)| (crate::types::TypeConstructor(parameter), ty))
                .collect();
            let applied = substituted_placeholders(&held.target, &fixed);
            found.push((
                Applies {
                    arguments: held
                        .arguments
                        .iter()
                        .map(|argument| substituted_placeholders(argument, &fixed))
                        .collect(),
                    open: held
                        .parameters
                        .iter()
                        .copied()
                        .filter(|parameter| {
                            !fixed.contains_key(&crate::types::TypeConstructor(parameter))
                        })
                        .collect(),
                },
                applied,
            ));
        }
        for bounded in &self.trait_bounds {
            if bounded.variable == *ty && bounded.trait_name == trait_name {
                found.push((
                    Applies {
                        arguments: bounded.arguments.clone(),
                        open: Vec::new(),
                    },
                    ty.clone(),
                ));
            }
        }
        // Of two implementations on one chain of narrowing, the narrower
        // type's is the one that applies: a List takes Seq's, not Data's.
        let narrowest: Vec<&(Applies, TypeRef)> = found
            .iter()
            .filter(|(_, wide)| {
                !found
                    .iter()
                    .any(|(_, narrow)| narrow != wide && narrow.is(wide) && !wide.is(narrow))
            })
            .collect();
        let mut kept: Vec<Applies> = Vec::new();
        for (applies, _) in narrowest {
            if !kept.iter().any(|held| held.arguments == applies.arguments) {
                kept.push(applies.clone());
            }
        }
        kept
    }

    /// The trait function a receiver of a type reaches by name: the trait,
    /// and the function's signature with `Self` and the trait's arguments in
    /// place, once for each set of arguments the type implements the trait
    /// at. Two traits offering one name are an ambiguity.
    /// The type of a slice of `receiver`: what [`sliced`] gives a built-in
    /// type, and the type itself for one implementing `Slice`.
    fn sliced(
        &self,
        receiver: &TypeRef,
        written: Option<Written>,
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        sliced(receiver, written).or_else(|message| {
            if self.applicable(receiver, "Slice").is_empty() {
                Err(Diagnostic::typing(span, message))
            } else {
                Ok(receiver.clone())
            }
        })
    }

    fn method_for(
        &self,
        receiver: &TypeRef,
        name: &str,
        span: &Range<usize>,
    ) -> Result<Option<(String, Vec<MethodSig>)>, Diagnostic> {
        let mut found: Vec<(String, Vec<MethodSig>)> = Vec::new();
        let mut names: Vec<&String> = self.traits.keys().collect();
        names.sort();
        for trait_name in names {
            let info = &self.traits[trait_name];
            let Some(signature) = info.methods.iter().find(|method| method.name == name) else {
                continue;
            };
            let signatures: Vec<MethodSig> = self
                .applicable(receiver, trait_name)
                .iter()
                .map(|applies| signature_in_place(signature, receiver, info, &applies.arguments))
                .collect();
            if !signatures.is_empty() {
                found.push((trait_name.clone(), signatures));
            }
        }
        match found.len() {
            0 => Ok(None),
            1 => Ok(found.pop()),
            _ => Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` is a function of {} for {receiver}, so the call is ambiguous",
                    found
                        .iter()
                        .map(|(trait_name, _)| format!("`{trait_name}`"))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            )),
        }
    }

    /// Hold a call to a bounded function to each of its trait bounds: the
    /// type each bound names implements the trait at the bound's arguments,
    /// and a parameter the call's arguments left open is filled from the one
    /// implementation that applies.
    fn resolve_bounds(
        &self,
        callee: &str,
        bounds: &[Bounded],
        bound: &mut HashMap<crate::types::TypeConstructor, TypeRef>,
        span: &Range<usize>,
    ) -> Result<(), Diagnostic> {
        for bounded in bounds {
            let filled = bound
                .get(&bounded.variable.constructor)
                .cloned()
                .unwrap_or(TypeRef::NEVER);
            if filled == TypeRef::NEVER {
                continue;
            }
            let applicable = self.applicable(&filled, &bounded.trait_name);
            let unmet = || {
                let wanted: Vec<TypeRef> = bounded
                    .arguments
                    .iter()
                    .map(|argument| substituted_placeholders(argument, bound))
                    .collect();
                Diagnostic::typing(
                    span.clone(),
                    format!(
                        "`{callee}` needs {} : {}, and {filled} does not implement `{}`",
                        bounded.name,
                        trait_written(&bounded.trait_name, &bounded.arguments),
                        trait_written(&bounded.trait_name, &wanted)
                    ),
                )
            };
            if applicable.is_empty() {
                return Err(unmet());
            }
            // A generic body reaches an implementation through its receiver
            // alone, so it cannot tell two of one trait for one type apart.
            let has_functions = self
                .traits
                .get(&bounded.trait_name)
                .is_some_and(|info| !info.methods.is_empty());
            if has_functions && applicable.len() > 1 {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!(
                        "`{callee}` needs {} : {}, and {filled} implements `{}` at several \
                         arguments, which a generic body does not tell apart",
                        bounded.name,
                        trait_written(&bounded.trait_name, &bounded.arguments),
                        bounded.trait_name
                    ),
                ));
            }
            let mut fills: Vec<HashMap<crate::types::TypeConstructor, TypeRef>> = Vec::new();
            for applies in &applicable {
                if let Some(fill) = fill_from(&bounded.arguments, applies, bound)
                    && !fills.contains(&fill)
                {
                    fills.push(fill);
                }
            }
            match fills.len() {
                0 => return Err(unmet()),
                1 => bound.extend(fills.pop().expect("one fill")),
                _ => {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!(
                            "`{callee}` needs {} : {}, and {filled} implements it at {}, which \
                             this call does not choose between",
                            bounded.name,
                            trait_written(&bounded.trait_name, &bounded.arguments),
                            applicable
                                .iter()
                                .map(|applies| trait_written(
                                    &bounded.trait_name,
                                    &applies.arguments
                                ))
                                .collect::<Vec<_>>()
                                .join(" and ")
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// `Point(x = 1, y = 2)`: a record built from its fields, by name or in
    /// declaration order, with the record's parameters taken from the values.
    fn construct_record(
        &mut self,
        name: &str,
        shape: &crate::types::program::RecordShape,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        if let Some(module) = crate::declarations::hidden(name) {
            return Err(crate::declarations::private_type(name, &module, &span));
        }
        let mut given: Vec<Option<&Arg>> = vec![None; shape.fields.len()];
        for (index, argument) in arguments.iter().enumerate() {
            let position = match &argument.name {
                Some(written) => shape
                    .fields
                    .iter()
                    .position(|field| field.name == *written)
                    .ok_or_else(|| {
                        Diagnostic::name(
                            argument.value.span.clone(),
                            format!("`{name}` has no field `{written}`"),
                        )
                    })?,
                None if index < shape.fields.len() => index,
                None => {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!(
                            "`{name}` has {} field{}, and this is one more",
                            shape.fields.len(),
                            if shape.fields.len() == 1 { "" } else { "s" }
                        ),
                    ));
                }
            };
            if given[position].replace(argument).is_some() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!(
                        "`{}` is supplied twice to `{name}`",
                        shape.fields[position].name
                    ),
                ));
            }
        }
        let mut bound: HashMap<&'static str, TypeRef> = HashMap::new();
        for (field, argument) in shape.fields.iter().zip(given) {
            let Some(argument) = argument else {
                if field.optional {
                    continue;
                }
                return Err(Diagnostic::typing(
                    span,
                    format!("`{name}` needs `{}`", field.name),
                ));
            };
            let open = crate::declarations::mentions_parameter(&field.ty, &shape.parameters);
            let expected = (!open).then(|| field.ty.clone());
            let actual = self.expecting(&argument.value, expected.as_ref())?;
            // An optional field takes its type or an Option of it.
            let actual = if field.optional && actual.constructor.0 == "Option" {
                actual.present().clone()
            } else {
                actual
            };
            if open {
                if !unify(&field.ty, &actual, &shape.parameters, &mut bound) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!(
                            "`{}` of `{name}` is {}, and this is {actual}",
                            field.name, field.ty
                        ),
                    ));
                }
            } else if !actual.is(&field.ty) {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!(
                        "`{}` of `{name}` is {}, and this is {actual}",
                        field.name, field.ty
                    ),
                ));
            }
        }
        let args: Vec<TypeRef> = shape
            .parameters
            .iter()
            .map(|parameter| bound.get(parameter).cloned().unwrap_or(TypeRef::NEVER))
            .collect();
        let system = crate::types::builtin::system()
            .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
        let ty = system
            .apply(crate::types::program::constructor(name), args)
            .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
        self.checked.records.insert(span, ty.clone());
        Ok(ty)
    }

    /// `xs | f` where `f` is an `Elementwise<I, O>` and `xs` a collection of
    /// `I`: the step runs `f` over each element, giving the collection of
    /// `O` that `xs | map(f)` gives. `None` when the input is not such a
    /// collection, and the step applies `f` to it whole.
    fn over_each(
        &mut self,
        callee: &Expr,
        input: &TypeRef,
        span: &Range<usize>,
    ) -> Result<Option<TypeRef>, Diagnostic> {
        let Kind::Ident(name) = &callee.kind else {
            return Ok(None);
        };
        let Some(declared) = self.scope.get(name).cloned() else {
            return Ok(None);
        };
        if declared.operation_shape() != Some(crate::extensions::Shape::Elementwise) {
            return Ok(None);
        }
        let (element_in, element_out) = (declared.args[0].clone(), declared.args[1].clone());
        if input.is(&element_in) {
            return Ok(None);
        }
        let Some(element) = input.iterated() else {
            return Ok(None);
        };
        if !element.is(&element_in) {
            return Err(Diagnostic::typing(
                callee.span.clone(),
                format!(
                    "`{name}` is {declared}, and runs over each element of a collection of \
                     {element_in}; this input is {input}"
                ),
            ));
        }
        self.expression(callee)?;
        let effectful = self.effectful.contains_key(name);
        self.using(name, effectful);
        self.user_step(callee, span, true);
        Ok(Some(input.image(element_out)))
    }

    /// Record a step whose operation is a function value, with the shape
    /// and purity a planner reads off its type.
    fn user_step(&mut self, callee: &Expr, span: &Range<usize>, over_each: bool) {
        let key = self.callee_key(callee);
        let ty = key.as_ref().and_then(|key| self.scope.get(key));
        let shape = ty
            .and_then(TypeRef::operation_shape)
            .unwrap_or(crate::extensions::Shape::Barrier);
        let purity = if key
            .as_ref()
            .is_some_and(|key| self.effectful.contains_key(key))
        {
            Purity::Effectful
        } else {
            Purity::Pure
        };
        self.checked.user_steps.insert(
            span.clone(),
            (
                crate::execution::OperationType {
                    name: key.unwrap_or_else(|| "<function>".to_owned()),
                    shape,
                    purity,
                    presentation: false,
                },
                over_each,
            ),
        );
    }

    /// Whether a name is bound to a function value.
    fn names_function(&self, expression: &Expr) -> bool {
        match &expression.kind {
            Kind::Ident(name) => self
                .scope
                .get(name)
                .is_some_and(|ty| ty.as_function().is_some()),
            Kind::Field(..) => self
                .callee_key(expression)
                .is_some_and(|key| self.scope.contains_key(&key)),
            _ => false,
        }
    }

    fn expression(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic> {
        let ty = self.infer_expression(expression)?;
        self.checked
            .types
            .insert(expression.span.clone(), ty.clone());
        Ok(ty)
    }

    fn infer_expression(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic> {
        let span = expression.span.clone();
        match &expression.kind {
            Kind::Block(_) => Err(Diagnostic::syntax(
                span,
                "a block is written only as the body of a user `fn`",
            )),
            Kind::List(values) | Kind::Set(values) => {
                // `[[alice]]` with `alice` unbound is a reference in the old
                // spelling: name the one it now has.
                if let (Kind::List(_), [only]) = (&expression.kind, values.as_slice())
                    && let Kind::List(inner) = &only.kind
                    && let [element] = inner.as_slice()
                    && let Kind::Ident(name) = &element.kind
                    && !self.scope.contains_key(name)
                    && !builtins::exists(name)
                {
                    return Err(Diagnostic::name(
                        element.span.clone(),
                        format!(
                            "`{name}` is not bound; a document reference is written `@[[{name}]]`"
                        ),
                    ));
                }
                let mut element = TypeRef::NEVER;
                for value in values {
                    let actual = self.expression(value)?;
                    element = element.common_type(&actual).ok_or_else(|| {
                        Diagnostic::typing(
                            value.span.clone(),
                            format!(
                                "collection elements {element} and {actual} have no common type"
                            ),
                        )
                    })?;
                }
                Ok(if matches!(expression.kind, Kind::Set(_)) {
                    TypeRef::set(element)
                } else {
                    TypeRef::list(element)
                })
            }
            Kind::Construct {
                annotation,
                arguments,
            } => {
                let expected = crate::declarations::annotation(annotation)?;
                crate::constructors::check(
                    self,
                    expected.constructor,
                    arguments,
                    Some(&expected),
                    span,
                )
            }
            Kind::Int(_) => Ok(TypeRef::INT),
            Kind::Float(_) => Ok(TypeRef::FLOAT),
            Kind::Bool(_) => Ok(TypeRef::BOOL),
            Kind::Str(_) => Ok(TypeRef::STR),
            Kind::Data(entries) => {
                for (_, value) in entries {
                    self.expression(value)?;
                }
                Ok(TypeRef::DATA)
            }
            // A reference may not resolve — a forward link to a document
            // nobody has written is ordinary — so it is optional by type.
            Kind::DocRef(_) => Ok(TypeRef::optional(TypeRef::CARD)),
            // Absence of nothing in particular: an `Option<Never>`, which is
            // an `Option<T>` for every `T`.
            Kind::Ident(name) if name == NONE && !self.scope.contains_key(name) => {
                Ok(TypeRef::optional(TypeRef::NEVER))
            }
            Kind::Ident(name) if name == "_" => Err(Diagnostic::syntax(
                span,
                "`_` is a hole only as a whole argument of a call, `f(a, _)`",
            )),
            Kind::Ident(name) => self.scope.get(name).cloned().ok_or_else(|| {
                // Unbound only because no vault was given: say what to pass,
                // rather than that the name does not exist.
                // `vault` and the program module's bindings over it exist only when
                // a vault is given, and only in the program module.
                let hosted = name == VAULT
                    || crate::extensions::io::BINDINGS
                        .iter()
                        .any(|(bound, _)| bound == name)
                    || crate::library::bindings()
                        .iter()
                        .any(|(bound, _)| bound == name);
                if self.module_file && hosted {
                    Diagnostic::name(
                        span,
                        format!(
                            "`{name}` is not bound in a module: the host seeds it into the \
                             program module alone, so a module takes what it needs from a \
                             program as a parameter"
                        ),
                    )
                } else if name == VAULT
                    || crate::library::bindings()
                        .iter()
                        .any(|(bound, _)| bound == name)
                {
                    Diagnostic::name(
                        span,
                        format!(
                            "`{name}` needs a vault: run inside one or pass --vault <directory>"
                        ),
                    )
                } else if let Some(ty) = self.externals.get(name) {
                    Diagnostic::name(
                        span,
                        format!(
                            "`{name}` is declared external here, `{name} : {ty}`, and only an \
                             extension implements it"
                        ),
                    )
                } else if builtins::exists(name) {
                    Diagnostic::typing(
                        span,
                        format!(
                            "`{name}` is a pipeline step and needs an input: write `… | {name}`"
                        ),
                    )
                } else {
                    let hint = builtins::hint(name);
                    Diagnostic::name(span, format!("`{name}` is not bound{hint}"))
                }
            }),
            Kind::Given {
                value,
                condition,
                otherwise,
            } => {
                let condition_type = self.expression(condition)?;
                if condition_type != TypeRef::BOOL {
                    return Err(Diagnostic::typing(
                        condition.span.clone(),
                        format!(
                            "a `given` condition is a Bool, not {condition_type}; \
                             an optional one says what absence means with `unwrap_or`"
                        ),
                    ));
                }
                let value_type = self.expression(value)?;
                match otherwise {
                    None => Ok(TypeRef::optional(value_type)),
                    Some(other) => {
                        let other_type = self.expression(other)?;
                        value_type.common_type(&other_type).ok_or_else(|| {
                            Diagnostic::typing(
                                span,
                                format!(
                                    "`given` gives {value_type} and `otherwise` {other_type}, \
                                     which have no common type"
                                ),
                            )
                        })
                    }
                }
            }
            Kind::Match { subject, arms } => self.match_expression(subject, arms, span),
            Kind::Tee { .. } => Err(Diagnostic::syntax(
                span,
                "`tee` is a pipeline step: write `input | tee name = branch`",
            )),
            Kind::Arith(..) => self.arithmetic(expression),
            Kind::Relate {
                left,
                right,
                relation,
            } => {
                let left_type = self.expression(left)?;
                let right_type = self.expression(right)?;
                // The rule `compare` has: one Orderable type on both sides,
                // with no widening and no looking through `Option`. `Never`,
                // the element of nothing, has no value to compare and so
                // stands beside any one.
                let never = left_type == TypeRef::NEVER || right_type == TypeRef::NEVER;
                let ordered = if left_type == TypeRef::NEVER {
                    right_type.is_orderable()
                } else {
                    left_type.is_orderable()
                };
                if (!never && left_type != right_type) || !ordered {
                    return Err(Diagnostic::typing(
                        span,
                        format!(
                            "`{}` requires two values of the same Orderable type, not {left_type} and {right_type}",
                            relation.symbol()
                        ),
                    ));
                }
                Ok(TypeRef::BOOL)
            }
            Kind::Logic(connective, ..) => {
                let mut operands = Vec::new();
                let mut first = expression;
                while let Kind::Logic(_, left, right) = &first.kind {
                    operands.push(right.as_ref());
                    first = left;
                }
                operands.push(first);
                for operand in operands.into_iter().rev() {
                    self.condition(operand, connective.symbol())?;
                }
                Ok(TypeRef::BOOL)
            }
            Kind::Not(operand) => {
                self.condition(operand, "not")?;
                Ok(TypeRef::BOOL)
            }
            Kind::Compare { left, right, .. } => {
                let left_type = self.expression(left)?;
                let right_type = self.expression(right)?;
                // A type implementing `Eq` decides its own equality.
                if left_type.constructor.0 != "Option"
                    && let Some((_, signatures)) = self.method_for(&left_type, "eq", &span)?
                    && self
                        .impls
                        .iter()
                        .any(|held| held.trait_name == "Eq" && left_type.is(&held.target))
                {
                    let parameter = signatures[0].parameters[0].clone();
                    if !right_type.is(&parameter) {
                        return Err(Diagnostic::typing(
                            span,
                            format!(
                                "`Eq` for {left_type} compares with {parameter}, not {right_type}"
                            ),
                        ));
                    }
                    self.checked.operator_calls.insert(span, "Eq");
                    return Ok(TypeRef::BOOL);
                }
                // An open tree's leaf may be compared with a literal: that is
                // what asking a header a question looks like. Equality looks
                // through `Option`, so a key that may be missing still can,
                // and through subtyping, so `Json` text compares with a string.
                let (left_type, right_type) = (left_type.present(), right_type.present());
                let comparable = left_type.is(right_type)
                    || right_type.is(left_type)
                    || *left_type == TypeRef::DATA
                    || *right_type == TypeRef::DATA;
                if !comparable {
                    return Err(Diagnostic::typing(
                        span,
                        format!("{left_type} and {right_type} are never equal"),
                    ));
                }
                Ok(TypeRef::BOOL)
            }
            Kind::Field(receiver, field) if matches!(&receiver.kind, Kind::Ident(module) if self.is_module(module)) =>
            {
                let Kind::Ident(module) = &receiver.kind else {
                    unreachable!("matched a module name");
                };
                let key = format!("{module}.{field}");
                let ty = self.scope.get(&key).cloned().ok_or_else(|| {
                    Diagnostic::name(
                        span.clone(),
                        format!("the module `{module}` has no function `{field}`"),
                    )
                })?;
                self.checked.module_refs.insert(span, key);
                Ok(ty)
            }
            Kind::Field(receiver, field) => {
                let receiver_type = self.expression(receiver)?;
                let ty = field_type(&receiver_type, field).ok_or_else(|| {
                    // A hit holds any value, so what it retrieved is `value`.
                    let hint = if receiver_type.present().constructor.0 == "Hit" && field == "card"
                    {
                        "; the retrieved value is `value`"
                    } else {
                        ""
                    };
                    Diagnostic::name(
                        span.clone(),
                        format!("{receiver_type} has no field `{field}`{hint}"),
                    )
                })?;
                let resolved =
                    crate::execution::Field::resolve(&receiver_type, field).ok_or_else(|| {
                        Diagnostic::name(
                            span.clone(),
                            format!("{receiver_type} has no field `{field}`"),
                        )
                    })?;
                self.checked.fields.insert(span, resolved);
                Ok(ty)
            }
            Kind::Index(receiver, index) => {
                let receiver_type = self.expression(receiver)?;
                let index_type = self.expression(index)?;
                // A Range in the bracket takes a slice, as `a:b` does.
                if index_type == TypeRef::RANGE {
                    return self.sliced(&receiver_type, None, span);
                }
                if index_type != TypeRef::INT {
                    return Err(Diagnostic::typing(
                        index.span.clone(),
                        format!("an index is an Int position or a Range, not {index_type}"),
                    ));
                }
                indexed(&receiver_type).or_else(|message| {
                    // Any other type takes a position through its
                    // implementation of `Index`, or a bound promising one.
                    match self.applicable(&receiver_type, "Index").as_slice() {
                        [applies] => Ok(TypeRef::optional(applies.arguments[0].clone())),
                        _ => Err(Diagnostic::typing(span, message)),
                    }
                })
            }
            Kind::Slice(receiver, start, end) => {
                let receiver_type = self.expression(receiver)?;
                for bound in [start, end].into_iter().flatten() {
                    let bound_type = self.expression(bound)?;
                    if bound_type != TypeRef::INT {
                        return Err(Diagnostic::typing(
                            bound.span.clone(),
                            format!("a bound of a range is an Int, not {bound_type}"),
                        ));
                    }
                }
                let negative = |bound: &Option<Box<Expr>>| {
                    bound
                        .as_deref()
                        .is_some_and(|bound| matches!(bound.kind, Kind::Int(value) if value < 0))
                };
                let written = Written {
                    from_end: negative(start) || negative(end),
                    bounded: end.is_some(),
                };
                self.sliced(&receiver_type, Some(written), span)
            }
            Kind::Link {
                source,
                target,
                data,
            } => {
                if let Some(data) = data {
                    self.expression(data)?;
                }
                for end in [source, target] {
                    let end_type = self.expression(end)?;
                    let resolved = end_type.present().clone();
                    if !resolved.is(&TypeRef::DOC) && resolved != TypeRef::STR {
                        return Err(Diagnostic::typing(
                            end.span.clone(),
                            format!(
                                "a link endpoint is a document reference, `@[[name]]`, not {end_type}"
                            ),
                        ));
                    }
                }
                Ok(TypeRef::EDGE)
            }
            Kind::Lambda { .. } => self.lambda_value(expression, None),
            Kind::Call { callee, arguments } => {
                if let Kind::Field(receiver, name) = &callee.kind {
                    return self.dot_call(callee, receiver, name, arguments, span);
                }
                if let Kind::Ident(name) = &callee.kind
                    && !self.constructor_signatures(name).is_empty()
                {
                    return self.construct_by_form(callee, name, arguments, span);
                }
                if self.names_function(callee) || !matches!(callee.kind, Kind::Ident(_)) {
                    return self.application(callee, None, arguments, span);
                }
                let Kind::Ident(name) = &callee.kind else {
                    unreachable!("a callee that is not a name is applied above");
                };
                if let Some(constructor) = crate::constructors::named(name) {
                    return crate::constructors::check(self, constructor, arguments, None, span);
                }
                if crate::types::program::is_record_valued(name)
                    && let Some(shape) = crate::types::program::record(name)
                {
                    return self.construct_record(name, &shape, arguments, span);
                }
                if (name == OK || name == ERR)
                    && !self.scope.contains_key(name)
                    && arguments.len() == 1
                {
                    let [argument] = arguments.as_slice() else {
                        unreachable!("one argument");
                    };
                    if argument.name.is_some() {
                        return Err(Diagnostic::typing(
                            span,
                            format!("`{name}` takes one positional value"),
                        ));
                    }
                    let held = self.expression(&argument.value)?;
                    return Ok(if name == OK {
                        TypeRef::result(held, TypeRef::NEVER)
                    } else {
                        TypeRef::result(TypeRef::NEVER, held)
                    });
                }
                if name == SOME && !self.scope.contains_key(name) {
                    let [argument] = arguments.as_slice() else {
                        return Err(Diagnostic::typing(span, "`some` takes one value"));
                    };
                    if argument.name.is_some() {
                        return Err(Diagnostic::typing(
                            span,
                            "`some` takes one positional value",
                        ));
                    }
                    return Ok(TypeRef::optional(self.expression(&argument.value)?));
                }
                if name == "compare" {
                    if arguments.len() != 2 || arguments.iter().any(|arg| arg.name.is_some()) {
                        return Err(Diagnostic::typing(
                            span,
                            "compare expects two positional values",
                        ));
                    }
                    let left = self.expression(&arguments[0].value)?;
                    let right = self.expression(&arguments[1].value)?;
                    if left != right || !left.is_orderable() {
                        return Err(Diagnostic::typing(
                            span,
                            "compare requires two values of the same Orderable type",
                        ));
                    }
                    return Ok(TypeRef::ORDERING);
                }
                // Two values combined, `min(a, b)` or `merge(m, n)`: the
                // combiner, where one value is the step of the same name.
                if let Some(combiner) = crate::extensions::combiner(name).map(|held| held.name)
                    && arguments.len() == 2
                    && arguments.iter().all(|argument| argument.name.is_none())
                    && !self.scope.contains_key(name)
                {
                    let left = self.expression(&arguments[0].value)?;
                    let right = self.expression(&arguments[1].value)?;
                    let ty = combined(combiner, &left, &right)
                        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
                    self.checked.combiners.insert(span.clone(), combiner);
                    return Ok(ty);
                }
                if builtins::exists(name) {
                    let Some((input, arguments)) = arguments.split_first() else {
                        return Err(Diagnostic::typing(span, format!("{name} needs an input")));
                    };
                    if input.name.is_some() {
                        return Err(Diagnostic::typing(
                            input.value.span.clone(),
                            "the input must be positional",
                        ));
                    }
                    let written = &input.value;
                    let input = self.expression(written)?;
                    let positional = arguments
                        .iter()
                        .filter(|argument| argument.name.is_none())
                        .count();
                    self.not_declared_below(name, &span)?;
                    let step = self.resolution.step_for(name, &input, positional, &span)?;
                    self.checked.steps.insert(span.clone(), step);
                    self.using(step.name, step.purity() == Purity::Effectful);
                    return self.step(step, &input, Some(written), arguments, span);
                }
                let _ = arguments;
                let hint = builtins::hint(name);
                Err(Diagnostic::name(
                    span,
                    format!("`{name}` is not bound{hint}"),
                ))
            }
            Kind::Pipe(input, step) => {
                let input_type = self.expression(input)?;
                // `x | tee name = branch`: the branch reads the input, its
                // result is bound to `name`, and the input passes on.
                if let Kind::Tee { name, branch } = &step.kind {
                    let result = self.with_bindings(
                        [(crate::ast::TEE_INPUT.to_owned(), input_type.clone())],
                        |checker| checker.expression(branch),
                    )?;
                    self.scope.insert(name.clone(), result);
                    return Ok(input_type);
                }
                // `x | f` and `x | f(a)` apply a bound function to the input.
                // The step's operation is the function's type: an operation
                // type states its shape, and a plain function is a barrier.
                // An `Elementwise<I, O>` over a collection of `I` runs over
                // each element, as `map` would.
                match &step.kind {
                    Kind::Ident(_) if self.names_function(step) => {
                        if let Some(produced) = self.over_each(step, &input_type, &span)? {
                            return Ok(produced);
                        }
                        let produced =
                            self.application(step, Some(&input_type), &[], span.clone())?;
                        self.user_step(step, &span, false);
                        return Ok(produced);
                    }
                    Kind::Call { callee, arguments } if self.names_function(callee) => {
                        let produced =
                            self.application(callee, Some(&input_type), arguments, span.clone())?;
                        self.user_step(callee, &span, false);
                        return Ok(produced);
                    }
                    _ => {}
                }
                let reference = step_of(step)?;
                let resolved = self.resolve(&reference, &input_type, &step.span)?;
                self.checked.steps.insert(span.clone(), resolved);
                self.using(resolved.name, resolved.purity() == Purity::Effectful);
                self.step(
                    resolved,
                    &input_type,
                    Some(input),
                    reference.arguments(),
                    step.span.clone(),
                )
            }
        }
    }

    /// Check a left-associated chain without consuming stack per operator.
    fn arithmetic(&mut self, mut expression: &Expr) -> Result<TypeRef, Diagnostic> {
        let mut operands = Vec::new();
        while let Kind::Arith(operator, left, right) = &expression.kind {
            operands.push((
                *operator,
                expression.span.clone(),
                left.as_ref(),
                right.as_ref(),
            ));
            expression = left;
        }
        let mut expected = self.expression(expression)?;
        for (operator, node, left, right) in operands.into_iter().rev() {
            // Arithmetic is homogeneous: there is no implicit widening.
            let refused = || format!("{} requires two Int or two Float operands", operator.noun());
            if matches!(expected.constructor.0, "Int" | "Float") {
                if self.expression(right)? != expected {
                    return Err(Diagnostic::typing(right.span.clone(), refused()));
                }
                continue;
            }
            // Any other type resolves the operator through its trait: a
            // declared type implementing `Add` has an addition of its own.
            let (trait_name, method) = operator_trait(operator);
            let Some((_, signatures)) = self.method_for(&expected, method, &node)? else {
                return Err(Diagnostic::typing(
                    left.span.clone(),
                    format!(
                        "{}, or two values of a type implementing `{trait_name}`, not {expected}",
                        refused()
                    ),
                ));
            };
            let signature = &signatures[0];
            let parameter = signature.parameters[0].clone();
            let actual = self.expecting(right, Some(&parameter))?;
            if !actual.is(&parameter) {
                return Err(Diagnostic::typing(
                    right.span.clone(),
                    format!("`{trait_name}` for {expected} takes {parameter}, not {actual}"),
                ));
            }
            expected = signature.result.clone();
            // The chain is lowered call by call, so each link's type is kept.
            self.checked.types.insert(node.clone(), expected.clone());
            self.checked.operator_calls.insert(node, trait_name);
        }
        Ok(expected)
    }

    /// Check an operand of a connective, which must be a plain `Bool`.
    fn condition(&mut self, operand: &Expr, connective: &str) -> Result<(), Diagnostic> {
        let ty = self.expression(operand)?;
        if ty != TypeRef::BOOL {
            return Err(Diagnostic::typing(
                operand.span.clone(),
                format!("`{connective}` requires a Bool operand, not {ty}"),
            ));
        }
        Ok(())
    }

    /// Check a lambda argument with its parameter bound to an element type.
    fn lambda_type(&mut self, argument: &Arg, types: Vec<TypeRef>) -> Result<TypeRef, Diagnostic> {
        let Kind::Lambda {
            parameters,
            annotations,
            body,
        } = &argument.value.kind
        else {
            // A function value where a lambda is written: it must accept what
            // the step gives it, so its parameters may be wider.
            let ty = self.expression(&argument.value)?;
            let Some((declared, result)) = ty.as_function() else {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!("expected a lambda or a function, found {ty}"),
                ));
            };
            if declared.len() != types.len() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!(
                        "expected a function of {} parameters, found {ty}",
                        types.len()
                    ),
                ));
            }
            for (given, declared) in types.iter().zip(declared) {
                if !given.is(declared) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("this function takes {declared} and is given {given}"),
                    ));
                }
            }
            return Ok(result.clone());
        };
        if parameters.len() != types.len() {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                format!("expected {} lambda parameters", types.len()),
            ));
        }
        for ((name, annotation), given) in parameters.iter().zip(annotations).zip(&types) {
            if let Some(annotation) = annotation {
                let declared = crate::declarations::annotation(annotation)?;
                if !given.is(&declared) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("parameter `{name}` is declared {declared} but receives {given}"),
                    ));
                }
            }
        }
        let previous = parameters
            .iter()
            .zip(types)
            .map(|(name, ty)| (name, self.scope.insert(name.clone(), ty)))
            .collect::<Vec<_>>();
        let result = self.expression(body);
        for (name, value) in previous.into_iter().rev() {
            match value {
                Some(value) => {
                    self.scope.insert(name.clone(), value);
                }
                None => {
                    self.scope.remove(name);
                }
            }
        }
        result
    }

    /// Which step a written reference names, given what is imported.
    ///
    /// A binding shadows the namespace and not the step: while `lexical` is
    /// bound, `lexical.rank` reads a field of that value, and the step stays
    /// reachable in its bare form.
    fn resolve(
        &self,
        reference: &StepRef<'_>,
        input: &TypeRef,
        span: &Range<usize>,
    ) -> Result<&'static Operation, Diagnostic> {
        match reference {
            StepRef::Bare { name, arguments } => {
                let positional = arguments
                    .iter()
                    .filter(|argument| argument.name.is_none())
                    .count();
                self.not_declared_below(name, span)?;
                self.resolution.step_for(name, input, positional, span)
            }
            StepRef::Qualified {
                extension, name, ..
            } => {
                if self.scope.contains_key(*extension) {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!(
                            "`{extension}` is bound to a value here, so `{extension}.{name}` \
                             reads a field rather than naming a step"
                        ),
                    ));
                }
                self.resolution.qualified(extension, name, span)
            }
        }
    }

    /// Check every arm against the subject's type, require the unguarded arms
    /// to cover every case, and type the match as the arms' common type.
    fn match_expression(
        &mut self,
        subject: &Expr,
        arms: &[Arm],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        let subject_type = self.expression(subject)?;
        let mut result = TypeRef::NEVER;
        for arm in arms {
            let mut bindings = Vec::new();
            pattern_bindings(&arm.pattern, &subject_type, &mut bindings)
                .map_err(|message| Diagnostic::typing(arm.span.clone(), message))?;
            let previous = bindings
                .into_iter()
                .map(|(name, ty)| {
                    let held = self.scope.insert(name.clone(), ty);
                    (name, held)
                })
                .collect::<Vec<_>>();
            let checked = (|| {
                if let Some(guard) = &arm.guard {
                    let guard_type = self.expression(guard)?;
                    if guard_type != TypeRef::BOOL {
                        return Err(Diagnostic::typing(
                            guard.span.clone(),
                            format!("a guard is a Bool, not {guard_type}"),
                        ));
                    }
                }
                self.expression(&arm.body)
            })();
            for (name, held) in previous.into_iter().rev() {
                match held {
                    Some(held) => {
                        self.scope.insert(name, held);
                    }
                    None => {
                        self.scope.remove(&name);
                    }
                }
            }
            let body = checked?;
            result = result.common_type(&body).ok_or_else(|| {
                Diagnostic::typing(
                    arm.body.span.clone(),
                    format!("match arms give {result} and {body}, which have no common type"),
                )
            })?;
        }
        let unguarded = arms
            .iter()
            .filter(|arm| arm.guard.is_none())
            .map(|arm| &arm.pattern)
            .collect::<Vec<_>>();
        if !exhaustive(&unguarded, &subject_type) {
            return Err(Diagnostic::typing(
                span,
                format!(
                    "this match does not cover every {subject_type}: {}",
                    missing_cases(&subject_type)
                ),
            ));
        }
        Ok(result)
    }

    /// Dispatch a pipeline step to the extension that provides it. A name
    /// a fold's `by` position does not resolve is suggested among the
    /// combiners, since that is the vocabulary the position takes.
    fn step(
        &mut self,
        step: &'static Operation,
        input: &TypeRef,
        input_expr: Option<&Expr>,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        let checked = self.dispatch_step(step, input, input_expr, arguments, span);
        if step.name != "reduce" {
            return checked;
        }
        checked.map_err(|refused| {
            let Diagnostic::Name { span: at, .. } = &refused else {
                return refused;
            };
            let unknown = arguments
                .iter()
                .find_map(|argument| match &argument.value.kind {
                    Kind::Ident(name) if argument.value.span == *at => Some(name),
                    _ => None,
                });
            match unknown {
                Some(name) => Diagnostic::name(
                    at.clone(),
                    format!("`{name}` is not bound{}", builtins::combiner_hint(name)),
                ),
                None => refused,
            }
        })
    }

    fn dispatch_step(
        &mut self,
        step: &'static Operation,
        input: &TypeRef,
        input_expr: Option<&Expr>,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        // A step applied to absence is a step applied to nothing, not a
        // failure: the reference that produced the absence was permitted. A
        // step sees the present value, unless only a declaration over the
        // option or the result itself takes it: `map` over one optional value
        // maps what is present, and over an optional list maps its elements.
        // An open tree is a list or not only at run time, so an optional one
        // is seen present, and a step over its elements is refused here.
        let lifted = input.present();
        let raw = step.takes_raw_input()
            || (*lifted != TypeRef::DATA
                && !step.specs().iter().any(|spec| spec.input.accepts(lifted))
                && step.specs().iter().any(|spec| spec.input.accepts(input)));
        let optional = !raw && input.constructor.0 == "Option";
        let input = if raw { input.clone() } else { lifted.clone() };
        let spec = step.overload(self, &input, arguments, &span)?;
        if let Some(id) = crate::extensions::binding::Environment::get()
            .ok()
            .and_then(|environment| environment.declaration_for(step, spec))
        {
            self.checked.declarations.insert(span.clone(), id);
        }
        let (result, call) = spec.check(step.name, self, &input, input_expr, arguments, &span)?;
        if let Some(check) = step.check {
            check(self, &call, span.clone())?;
        }
        // Absence is the empty collection to a step over a collection. A
        // step over anything else is lifted over it: absence gives absence,
        // and what the step produces is present.
        let empty = crate::types::collections::SET.apply([TypeRef::NEVER]);
        if optional && !spec.input.accepts(&empty) {
            self.checked.lifted.insert(span);
            return Ok(if result.constructor.0 == "Option" {
                result
            } else {
                TypeRef::optional(result)
            });
        }
        Ok(result)
    }
}

impl CheckCx for Checker<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn warn(&mut self, warning: crate::warnings::Warning) {
        // Choosing among overloads may read an argument twice, and says
        // what it finds once.
        if !self.checked.warnings.contains(&warning) {
            self.checked.warnings.push(warning);
        }
    }

    fn placeholder(&mut self, actual: &TypeRef, expected: Option<TypeRef>) -> bool {
        let Some(expected) = expected.filter(|expected| !expected.constructor.0.starts_with("%?"))
        else {
            return false;
        };
        match self.placeholders.get(&actual.constructor) {
            Some(held) => held.is(&expected) || expected.is(held),
            None => {
                self.placeholders.insert(actual.constructor, expected);
                true
            }
        }
    }

    fn function_arity(&mut self, argument: &Arg) -> Option<usize> {
        match &argument.value.kind {
            Kind::Lambda { parameters, .. } => Some(parameters.len()),
            Kind::Ident(name)
                if (crate::constructors::named(name).is_some() || builtins::exists(name))
                    && !self.scope.contains_key(name) =>
            {
                Some(1)
            }
            Kind::Ident(name) if !self.constructor_signatures(name).is_empty() => self
                .constructor_signatures(name)
                .first()
                .map(|(_, parameters)| parameters.len()),
            Kind::Ident(name) => self
                .scope
                .get(name)
                .and_then(|ty| ty.as_function().map(|(parameters, _)| parameters.len())),
            _ => None,
        }
    }

    fn infer(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic> {
        self.expression(expression)
    }

    fn lambda_parameters(
        &mut self,
        argument: &Arg,
        parameters: Vec<TypeRef>,
    ) -> Result<TypeRef, Diagnostic> {
        // A program's constructor passed where a function is takes the one
        // form whose first parameter takes what is passed.
        if let Kind::Ident(name) = &argument.value.kind
            && !self.constructor_signatures(name).is_empty()
        {
            return self.constructor_value(name, &argument.value.span, &parameters);
        }
        // A constructor passed where a function is: `map(Set)` builds a set
        // from each value it is given.
        if let Kind::Ident(name) = &argument.value.kind
            && let Some(constructor) = crate::constructors::named(name)
            && !self.scope.contains_key(name)
        {
            let span = argument.value.span.clone();
            if let [parameter] = parameters.as_slice() {
                crate::constructors::resolve_value(constructor, parameter, &span)?;
            }
            let [parameter] = parameters.as_slice() else {
                return Err(Diagnostic::typing(
                    span,
                    format!(
                        "`{name}` builds a value from one argument, and a function of {} is expected",
                        parameters.len()
                    ),
                ));
            };
            let held = Arg {
                name: None,
                value: Expr {
                    kind: Kind::Ident(CONSTRUCTED.to_owned()),
                    span: span.clone(),
                },
            };
            let result =
                self.with_bindings([(CONSTRUCTED.to_owned(), parameter.clone())], |checker| {
                    crate::constructors::check(
                        checker,
                        constructor,
                        std::slice::from_ref(&held),
                        None,
                        span.clone(),
                    )
                })?;
            self.checked
                .constructor_values
                .insert(span, (parameter.clone(), result.clone()));
            return Ok(result);
        }
        // A step named where a function is: `map(str)` applies the step to
        // each value it is given.
        if let Kind::Ident(name) = &argument.value.kind
            && builtins::exists(name)
            && !self.scope.contains_key(name)
        {
            let span = argument.value.span.clone();
            let [parameter] = parameters.as_slice() else {
                return Err(Diagnostic::typing(
                    span,
                    format!(
                        "`{name}` takes one input, and a function of {} is expected",
                        parameters.len()
                    ),
                ));
            };
            self.not_declared_below(name, &span)?;
            let step = self.resolution.step_for(name, parameter, 0, &span)?;
            self.using(step.name, step.purity() == Purity::Effectful);
            let produced = self.step(step, parameter, None, &[], span.clone())?;
            if let Some(function) = TypeRef::function(vec![parameter.clone()], produced.clone()) {
                self.checked.types.insert(span.clone(), function);
            }
            self.checked
                .step_values
                .insert(span, (step, parameter.clone(), produced.clone()));
            return Ok(produced);
        }
        if let Kind::Lambda {
            parameters: names, ..
        } = &argument.value.kind
        {
            for name in names {
                self.lint(crate::naming::Kind::Parameter, name, &argument.value.span);
            }
        }
        self.lambda_type(argument, parameters)
    }

    fn type_argument(&mut self, argument: &Arg) -> Result<TypeRef, Diagnostic> {
        let Kind::Ident(name) = &argument.value.kind else {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "expected a type name",
            ));
        };
        if let Some(module) = crate::declarations::hidden(name) {
            return Err(crate::declarations::private_type(
                name,
                &module,
                &argument.value.span,
            ));
        }
        // A constructor written bare, `is(List)`, asks for the constructor
        // whatever its arguments are.
        let ty = crate::types::named(name, &[])
            .or_else(|| {
                let constructor = crate::types::builtin::constructor(name)?;
                let arity = crate::types::builtin::system()
                    .ok()?
                    .definition(constructor)
                    .ok()?
                    .parameters
                    .len();
                (arity > 0).then(|| constructor.apply(vec![TypeRef::DATA; arity]))
            })
            .ok_or_else(|| {
                Diagnostic::name(
                    argument.value.span.clone(),
                    format!("unknown type `{name}`"),
                )
            })?;
        self.checked
            .types
            .insert(argument.value.span.clone(), ty.clone());
        self.checked
            .type_arguments
            .insert(argument.value.span.clone());
        Ok(ty)
    }
}

/// How a pipeline step was written: bare, or qualified by its extension.
pub(crate) enum StepRef<'a> {
    /// `| take(5)` — resolved against everything the program imported.
    Bare { name: &'a str, arguments: &'a [Arg] },
    /// `| lexical.lexical("…")` — always available for an imported
    /// extension, and the way a reader disambiguates by hand.
    Qualified {
        extension: &'a str,
        name: &'a str,
        arguments: &'a [Arg],
    },
}

impl<'a> StepRef<'a> {
    /// The arguments written at the call, which stay unevaluated syntax.
    pub(crate) fn arguments(&self) -> &'a [Arg] {
        match self {
            Self::Bare { arguments, .. } | Self::Qualified { arguments, .. } => arguments,
        }
    }
}

/// How a pipeline step was written.
pub(crate) fn step_of(step: &Expr) -> Result<StepRef<'_>, Diagnostic> {
    let not_a_step = || {
        Diagnostic::typing(
            step.span.clone(),
            "a pipeline step is a name or a call, optionally qualified by its extension",
        )
    };
    let (callee, arguments) = match &step.kind {
        Kind::Call { callee, arguments } => (callee.as_ref(), arguments.as_slice()),
        _ => (step, &[][..]),
    };
    match &callee.kind {
        Kind::Ident(name) => Ok(StepRef::Bare { name, arguments }),
        Kind::Field(receiver, name) => match &receiver.kind {
            Kind::Ident(extension) => Ok(StepRef::Qualified {
                extension,
                name,
                arguments,
            }),
            _ => Err(not_a_step()),
        },
        _ => Err(not_a_step()),
    }
}

/// The type of `xs[i]`: the element at a position, or its absence.
///
/// Only a sequence has positions. An optional receiver nests no deeper: an
/// element of a sequence that may be missing is one `Option`.
fn indexed(receiver: &TypeRef) -> Result<TypeRef, String> {
    if receiver.constructor.0 == "Option" {
        return indexed(receiver.present());
    }
    if *receiver == TypeRef::DATA {
        return Ok(TypeRef::optional(TypeRef::DATA));
    }
    // A String's positions are its characters, each read as text.
    if *receiver == TypeRef::STR {
        return Ok(TypeRef::optional(TypeRef::STR));
    }
    if receiver.is_sequence() {
        let element = receiver
            .element()
            .ok_or_else(|| format!("{receiver} has no element type"))?;
        return Ok(TypeRef::optional(element));
    }
    if receiver.is_stream() && !receiver.is_collection() {
        return Err(format!(
            "{receiver} may not end, so a position in it is not known; bound it with `take(n)`"
        ));
    }
    if crate::types::MapKind::of(receiver.constructor).is_some() {
        return Err(format!(
            "{receiver} has keys, not positions; read a value with `get`"
        ));
    }
    if receiver.is_collection() {
        return Err(format!(
            "{receiver} has no positions, so it is not indexed; `sort` it into one"
        ));
    }
    Err(format!("{receiver} is not indexed; a sequence is"))
}

/// What a written `a:b` says about its bounds, as far as checking can see:
/// whether one is a literal counted from the end, and whether the end is
/// given.
#[derive(Debug, Clone, Copy)]
struct Written {
    from_end: bool,
    bounded: bool,
}

/// The type of `xs[a:b]` over a built-in type: the receiver's own type,
/// since a part of a value is of the value's kind. A Stream's slice is
/// decided by its bounds, so it is taken only with a written `a:b`.
fn sliced(receiver: &TypeRef, written: Option<Written>) -> Result<TypeRef, String> {
    if receiver.constructor.0 == "Option" {
        return sliced(receiver.present(), written).map(TypeRef::optional);
    }
    if *receiver == TypeRef::STR || *receiver == TypeRef::DATA {
        return Ok(receiver.clone());
    }
    if receiver.is_stream() && !receiver.is_collection() {
        let element = receiver
            .args
            .first()
            .cloned()
            .ok_or_else(|| format!("{receiver} has no element type"))?;
        return match written {
            None => Err(format!(
                "{receiver} is sliced by a written `a:b`, whose end decides whether the slice ends"
            )),
            Some(written) if written.from_end => Err(format!(
                "{receiver} may not end, so a position counted from its end is not known"
            )),
            Some(written) if written.bounded => Ok(TypeRef::list(element)),
            Some(_) => Ok(receiver.clone()),
        };
    }
    if receiver.is_sequence() {
        return Ok(receiver.clone());
    }
    if crate::types::MapKind::of(receiver.constructor).is_some() {
        return Err(format!(
            "{receiver} has keys, not positions; read a value with `get`"
        ));
    }
    if receiver.is_collection() {
        return Err(format!(
            "{receiver} has no positions, so it is not sliced; `sort` it into one"
        ));
    }
    Err(format!(
        "{receiver} is not sliced; a sequence, a String, Data or a type implementing Slice is"
    ))
}

/// The type of a field on a value of a type.
pub(crate) fn field_type(receiver: &TypeRef, field: &str) -> Option<TypeRef> {
    if field == "keys" && crate::types::MapKind::of(receiver.constructor).is_some() {
        let system = crate::types::builtin::system().ok()?;
        return crate::types::collections::keys_type(&system, receiver).ok();
    }
    match receiver.constructor.0 {
        // Absence nests no deeper than once: a field of a field that may be
        // missing is one `Option`, not two.
        "Option" => field_type(receiver.args.first()?, field)
            .map(|found| TypeRef::optional(found.present().clone())),
        // A key of an open tree may not be there, so reading one is optional
        // by type: `field? : T` and `Option<T>` are the same claim.
        "Data" => Some(TypeRef::optional(TypeRef::DATA)),
        // A record's fields are what its declaration says, in the program
        // or in `std/`.
        _ => crate::types::program::field_type(receiver, field)
            .or_else(|| crate::library::field_type(receiver, field)),
    }
}

/// The name an absent option is written with when nothing binds it.
const NONE: &str = "none";
/// The name a present option is built with when nothing binds it.
const SOME: &str = "some";
/// The names a result is built with when nothing binds them.
const OK: &str = "ok";
const ERR: &str = "err";

/// The names a pattern binds, at their types, or why it cannot fit `subject`.
fn pattern_bindings(
    pattern: &Pattern,
    subject: &TypeRef,
    bindings: &mut Vec<(String, TypeRef)>,
) -> Result<(), String> {
    let literal = |ty: TypeRef| {
        if subject.declared_common_type(&ty).is_some() {
            Ok(())
        } else {
            Err(format!("a {ty} pattern cannot fit a {subject}"))
        }
    };
    match pattern {
        Pattern::Wildcard => Ok(()),
        Pattern::Bind(name) => {
            bindings.push((name.clone(), subject.clone()));
            Ok(())
        }
        Pattern::Int(_) => literal(TypeRef::INT),
        Pattern::Float(_) => literal(TypeRef::FLOAT),
        Pattern::Str(_) => literal(TypeRef::STR),
        Pattern::Bool(_) => literal(TypeRef::BOOL),
        Pattern::Some(inner) if subject.constructor.0 == "Option" => {
            pattern_bindings(inner, subject.present(), bindings)
        }
        Pattern::None if subject.constructor.0 == "Option" => Ok(()),
        Pattern::Some(_) | Pattern::None => Err(format!(
            "`some` and `none` fit an Option, and this is a {subject}"
        )),
        Pattern::Ok(inner) if let Some((value, _)) = subject.as_result() => {
            pattern_bindings(inner, value, bindings)
        }
        Pattern::Err(inner) if let Some((_, error)) = subject.as_result() => {
            pattern_bindings(inner, error, bindings)
        }
        Pattern::Ok(_) | Pattern::Err(_) => Err(format!(
            "`ok` and `err` fit a Result, and this is a {subject}"
        )),
        Pattern::Type(name, inner) => {
            let wanted =
                crate::types::named(name, &[]).ok_or_else(|| format!("unknown type `{name}`"))?;
            if !crate::types::program::may_be(subject, &wanted) {
                return Err(format!(
                    "no {subject} can be a {wanted}, so this arm never fits"
                ));
            }
            pattern_bindings(inner, &wanted, bindings)
        }
    }
}

/// Whether the unguarded patterns cover every value of `subject`.
fn exhaustive(patterns: &[&Pattern], subject: &TypeRef) -> bool {
    let covers = |pattern: &&Pattern| match pattern {
        Pattern::Type(name, _) => {
            crate::types::named(name, &[]).is_some_and(|wanted| subject.is(&wanted))
        }
        other => other.is_irrefutable(),
    };
    if patterns.iter().any(covers) {
        return true;
    }
    // A union is covered when each of its members is.
    if let Ok(system) = crate::types::builtin::system()
        && let Ok(definition) = system.definition(subject.constructor)
        && let crate::types::TypeKind::Union(members) = &definition.kind
    {
        return members.iter().all(|member| exhaustive(patterns, member));
    }
    if let Some((value, error)) = subject.as_result() {
        let oks = patterns
            .iter()
            .filter_map(|pattern| match pattern {
                Pattern::Ok(inner) => Some(inner.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let errs = patterns
            .iter()
            .filter_map(|pattern| match pattern {
                Pattern::Err(inner) => Some(inner.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        return (*value == TypeRef::NEVER || exhaustive(&oks, value))
            && (*error == TypeRef::NEVER || exhaustive(&errs, error));
    }
    if subject.constructor.0 == "Option" {
        let present = patterns
            .iter()
            .filter_map(|pattern| match pattern {
                Pattern::Some(inner) => Some(inner.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        return patterns.contains(&&Pattern::None) && exhaustive(&present, subject.present());
    }
    if *subject == TypeRef::BOOL {
        return patterns.contains(&&Pattern::Bool(true))
            && patterns.contains(&&Pattern::Bool(false));
    }
    false
}

/// What an exhaustive match over `subject` needs, for the diagnostic.
fn missing_cases(subject: &TypeRef) -> &'static str {
    if subject.constructor.0 == "Option" {
        "cover both `some(…)` and `none`, or add `_`"
    } else if subject.as_result().is_some() {
        "cover both `ok(…)` and `err(…)`, or add `_`"
    } else if *subject == TypeRef::BOOL {
        "cover both `true` and `false`, or add `_`"
    } else if crate::types::builtin::system()
        .ok()
        .and_then(|system| system.definition(subject.constructor).ok().cloned())
        .is_some_and(|definition| matches!(definition.kind, crate::types::TypeKind::Union(_)))
    {
        "cover every member of the union with a type pattern, or add `_`"
    } else {
        "add a `_` arm"
    }
}
