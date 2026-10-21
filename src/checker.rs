//! Static checking: what a program's type is, before anything is evaluated.

use crate::ast::{Arg, Arm, Expr, FnDecl, ImplDecl, Kind, Pattern, Program, Stmt, TraitDecl};
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
    Checker {
        vault,
        scope,
        resolution: Resolution::new(&vault.extensions)?,
        declarations: Declarations::default(),
        checked: Checked::default(),
        variadic: HashMap::new(),
        parameter_names: HashMap::new(),
        generic_functions: HashMap::new(),
        placeholders: HashMap::new(),
        uses: Vec::new(),
        effectful: HashMap::new(),
        traits: core_traits(),
        trait_decls: HashMap::new(),
        impls: Vec::new(),
        trait_bounds: Vec::new(),
        trait_bounded: HashMap::new(),
        modules: Vec::new(),
        module_cache: ModuleCache::default(),
        loading: std::sync::Arc::default(),
        module_file: false,
        receiver: None,
        pattern_variables: None,
        seen_body: false,
    }
    .program(program)
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
    let mut checker = Checker {
        vault,
        scope: {
            let mut scope = globals(vault);
            scope.extend(bound.iter().cloned());
            scope
        },
        resolution,
        declarations: Declarations::default(),
        checked: Checked::default(),
        variadic: HashMap::new(),
        parameter_names: HashMap::new(),
        generic_functions: HashMap::new(),
        placeholders: HashMap::new(),
        uses: Vec::new(),
        effectful: HashMap::new(),
        traits: core_traits(),
        trait_decls: HashMap::new(),
        impls: Vec::new(),
        trait_bounds: Vec::new(),
        trait_bounded: HashMap::new(),
        modules: Vec::new(),
        module_cache: ModuleCache::default(),
        loading: std::sync::Arc::default(),
        module_file: false,
        receiver: None,
        pattern_variables: None,
        seen_body: false,
    };
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

/// A function named in `UpperCamelCase` is a type's constructor, which only
/// the module declaring the type provides.
fn constructor_name(name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
    if name.chars().next().is_some_and(char::is_uppercase) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!(
                "`{name}` is named like a type's constructor, which only the module declaring \
                 the type provides; name the function in snake_case"
            ),
        ));
    }
    Ok(())
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

/// The names every program over a vault starts with: `vault`, and the
/// library's bindings over it, `cards` among them.
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

/// The standard library's bindings, each checked as its own program in a
/// scope that holds only `vault`. Without a vault there are none.
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
            let typed = Checker {
                vault,
                scope: base(vault),
                resolution: Resolution::new(&vault.extensions).ok()?,
                declarations: Declarations::default(),
                checked: Checked::default(),
                variadic: HashMap::new(),
                parameter_names: HashMap::new(),
                generic_functions: HashMap::new(),
                placeholders: HashMap::new(),
                uses: Vec::new(),
                effectful: HashMap::new(),
                traits: core_traits(),
                trait_decls: HashMap::new(),
                impls: Vec::new(),
                trait_bounds: Vec::new(),
                trait_bounded: HashMap::new(),
                modules: Vec::new(),
                module_cache: ModuleCache::default(),
                loading: std::sync::Arc::default(),
                module_file: false,
                receiver: None,
                pattern_variables: None,
                seen_body: false,
            }
            .program(&program)
            .ok()?;
            Some((name, typed))
        })
        .collect()
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
    /// Every implementation the program has declared.
    impls: Vec<ImplInfo>,
    /// The trait bounds in force while a generic body is checked: each type
    /// variable and the trait it satisfies.
    trait_bounds: Vec<(TypeRef, String)>,
    /// Each generic function's trait bounds: the parameter name, its type
    /// variable and the trait, checked at every call.
    trait_bounded: HashMap<String, Vec<(String, TypeRef, String)>>,
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
}

type ModuleCache = std::sync::Arc<
    std::sync::Mutex<HashMap<String, std::sync::Arc<crate::modules::WorkspaceModule>>>,
>;

/// A trait's contract: its functions, with `Self` standing for the
/// implementing type.
#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    pub methods: Vec<MethodSig>,
}

/// One function of a trait.
#[derive(Debug, Clone)]
pub(crate) struct MethodSig {
    pub name: String,
    /// The parameters after `self`, written over `Self`.
    pub parameters: Vec<TypeRef>,
    pub result: TypeRef,
}

/// One implementation of a trait for a type.
#[derive(Debug, Clone)]
pub(crate) struct ImplInfo {
    pub trait_name: String,
    pub target: TypeRef,
    pub span: Range<usize>,
}

/// The type `Self` stands for inside a trait, before an implementation
/// binds it.
pub(crate) const SELF: crate::types::TypeConstructor = crate::types::TypeConstructor("%Self");

/// A type written over `Self`, with the implementing type in its place.
fn with_self(written: &TypeRef, target: &TypeRef) -> TypeRef {
    if written.constructor == SELF {
        return target.clone();
    }
    written
        .constructor
        .apply(written.args.iter().map(|arg| with_self(arg, target)))
}

/// The traits the language owns, in scope in every program: the operators
/// resolve through them, and `std/core.hql` restates them.
pub(crate) fn core_traits() -> HashMap<String, TraitInfo> {
    let self_type = SELF.apply([]);
    let binary = |name: &str, result: TypeRef| TraitInfo {
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
    ]
    .into_iter()
    .map(|(name, info)| (name.to_owned(), info))
    .collect()
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
    fn program(&mut self, program: &'a Program) -> Result<TypedProgram, Diagnostic> {
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
            self.checked
                .warnings
                .push(crate::warnings::Warning::new(span.clone(), message));
        }
    }

    fn statement(&mut self, statement: &'a Stmt) -> Result<TypeRef, Diagnostic> {
        if !matches!(statement, Stmt::Import { .. } | Stmt::ModuleFor { .. }) {
            self.seen_body = true;
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
                } => {
                    self.lint(crate::naming::Kind::Binding, name, name_span);
                    let expected = annotation
                        .as_ref()
                        .map(crate::declarations::annotation)
                        .transpose()?;
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
                    if !self.resolution.import(&dotted, alias.as_deref(), span)? {
                        self.import_module(&dotted, &local, span)?;
                    }
                    TypeRef::UNIT
                }
                Stmt::ModuleFor { pattern, span } => {
                    if !self.module_file {
                        return Err(Diagnostic::syntax(
                            span.clone(),
                            "`module for` opens a module file; a program has no receiver",
                        ));
                    }
                    if self.seen_body || self.receiver.is_some() {
                        return Err(Diagnostic::syntax(
                            span.clone(),
                            "`module for` is the first statement of a module file, written once",
                        ));
                    }
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
                    self.declaring(&declaration.name, |checker| checker.function(declaration))?;
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
        self.expression(expression)
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
    fn function(&mut self, declaration: &FnDecl) -> Result<(), Diagnostic> {
        constructor_name(&declaration.name, &declaration.span)?;
        self.lint(
            crate::naming::Kind::Function,
            &declaration.name,
            &declaration.span,
        );
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
            // the body.
            let trait_bound = written
                .filter(|bound| bound.arguments.is_empty() && self.traits.contains_key(&bound.name))
                .map(|bound| bound.name.clone());
            let bound = match (&trait_bound, written) {
                (None, Some(bound)) => Some(crate::declarations::annotation(bound)?),
                _ => None,
            };
            if let Some(trait_name) = &trait_bound
                && let Some(variable) = crate::types::builtin::type_variable(name, None)
            {
                traits_bounding.push((name.clone(), variable, trait_name.clone()));
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
        if variables.is_empty() {
            self.generic_functions.remove(&declaration.name);
        } else {
            self.generic_functions
                .insert(declaration.name.clone(), variables);
        }
        if traits_bounding.is_empty() {
            self.trait_bounded.remove(&declaration.name);
        } else {
            self.trait_bounded
                .insert(declaration.name.clone(), traits_bounding.clone());
        }
        let bounds_before = self.trait_bounds.len();
        self.trait_bounds.extend(
            traits_bounding
                .iter()
                .map(|(_, variable, trait_name)| (variable.clone(), trait_name.clone())),
        );
        let checked = self.function_body(declaration);
        self.trait_bounds.truncate(bounds_before);
        checked
    }

    /// The signature and body of a declared function, with its type
    /// variables and trait bounds in force.
    fn function_body(&mut self, declaration: &FnDecl) -> Result<(), Diagnostic> {
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
        self.scope.insert(declaration.name.clone(), ty.clone());
        if declaration
            .parameters
            .last()
            .is_some_and(|last| last.variadic)
        {
            self.variadic
                .insert(declaration.name.clone(), parameters.len());
        } else {
            self.variadic.remove(&declaration.name);
        }
        self.checked.functions.insert(declaration.span.clone(), ty);
        self.parameter_names
            .insert(declaration.name.clone(), names.clone());
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
        constructor_name(&alias.name, &alias.span)?;
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
                let element = parameters[fixed].element().unwrap_or(TypeRef::DATA);
                for (actual, at) in &supplied[fixed..] {
                    if !actual.is(&element) {
                        return Err(Diagnostic::typing(
                            at.clone(),
                            format!("a trailing argument here is {element}, not {actual}"),
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
            if let Some(bounds) = self.trait_bounded.get(callee_name).cloned() {
                for (name, variable, trait_name) in bounds {
                    let filled = bound
                        .get(&variable.constructor)
                        .cloned()
                        .unwrap_or(TypeRef::NEVER);
                    if !self.implements(&filled, &trait_name) {
                        return Err(Diagnostic::typing(
                            span,
                            format!(
                                "`{callee_name}` needs {name} : {trait_name}, and {filled} does \
                                 not implement `{trait_name}`"
                            ),
                        ));
                    }
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
            self.using(step.name, step.purity == Purity::Effectful);
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
        // promised by a bound on it.
        if let Some((trait_name, signature)) = self.method_for(&receiver_type, name, &span)? {
            let parameters: Vec<TypeRef> = signature
                .parameters
                .iter()
                .map(|parameter| with_self(parameter, &receiver_type))
                .collect();
            if arguments.len() != parameters.len() {
                return Err(Diagnostic::typing(
                    span,
                    format!(
                        "`{name}` of `{trait_name}` takes {} argument{} after the receiver, not {}",
                        parameters.len(),
                        if parameters.len() == 1 { "" } else { "s" },
                        arguments.len()
                    ),
                ));
            }
            for (argument, parameter) in arguments.iter().zip(&parameters) {
                if argument.name.is_some() {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        "a trait's function takes its arguments by position",
                    ));
                }
                let actual = self.expecting(&argument.value, Some(parameter))?;
                if !actual.is(parameter) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("this argument is {actual}, and `{name}` takes {parameter}"),
                    ));
                }
            }
            self.checked
                .method_calls
                .insert(span, (trait_name, name.to_owned()));
            return Ok(with_self(&signature.result, &receiver_type));
        }
        if let Some(&(combiner, _)) = crate::ast::COMBINERS.iter().find(|(held, _)| held == &name)
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
            let step = self
                .resolution
                .step_for(name, &receiver_type, positional, &span)?;
            self.checked.steps.insert(span.clone(), step);
            self.checked.receiver_calls.insert(span.clone());
            self.using(step.name, step.purity == Purity::Effectful);
            return self.step(step, &receiver_type, Some(receiver), arguments, span);
        }
        let hint = builtins::hint(name);
        Err(Diagnostic::name(
            span,
            format!("{receiver_type} has no function `{name}`{hint}"),
        ))
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
        for (trait_name, info) in &module.traits {
            self.traits
                .entry(trait_name.clone())
                .or_insert_with(|| info.clone());
        }
        for held in &module.impls {
            if !self.impls.iter().any(|known| {
                known.trait_name == held.trait_name
                    && known.target.constructor == held.target.constructor
            }) {
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
            vault: self.vault,
            scope: globals(self.vault),
            resolution: Resolution::new(&self.vault.extensions)?,
            declarations: Declarations::default(),
            checked: Checked::default(),
            variadic: HashMap::new(),
            parameter_names: HashMap::new(),
            generic_functions: HashMap::new(),
            placeholders: HashMap::new(),
            uses: Vec::new(),
            effectful: HashMap::new(),
            traits: core_traits(),
            trait_decls: HashMap::new(),
            impls: Vec::new(),
            trait_bounds: Vec::new(),
            trait_bounded: HashMap::new(),
            modules: Vec::new(),
            module_cache: std::sync::Arc::clone(&self.module_cache),
            loading: std::sync::Arc::clone(&self.loading),
            module_file: true,
            receiver: None,
            pattern_variables: None,
            seen_body: false,
        };
        let checked = checker.program(&program);
        self.loading
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop();
        let typed = checked.map_err(inside)?;
        let mut functions = Vec::new();
        for statement in &program.statements {
            let Stmt::Fn(declaration) = statement else {
                continue;
            };
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
        let core = core_traits();
        let traits = checker
            .traits
            .iter()
            .filter(|(name, _)| !core.contains_key(*name))
            .map(|(name, info)| (name.clone(), info.clone()))
            .collect();
        let module = std::sync::Arc::new(crate::modules::WorkspaceModule {
            name: name.to_owned(),
            receiver: checker.receiver.clone(),
            functions,
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

    /// The function a receiver module provides for a receiver of a type: the
    /// modules whose pattern the type fits, and among them the one declaring
    /// `name` with `self`. Two such modules are an ambiguity.
    fn receiver_function(
        &self,
        receiver: &TypeRef,
        name: &str,
        span: &Range<usize>,
    ) -> Result<Option<(String, TypeRef)>, Diagnostic> {
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
        match found.len() {
            0 => Ok(None),
            1 => Ok(found.pop()),
            _ => Err(Diagnostic::name(
                span.clone(),
                format!(
                    "`{name}` over {receiver} is declared by {}, so the call is ambiguous; \
                     qualify it",
                    found
                        .iter()
                        .map(|(key, _)| format!("`{key}`"))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            )),
        }
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
            let same = core.methods.len() == methods.len()
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
        self.traits
            .insert(declaration.name.clone(), TraitInfo { methods });
        self.trait_decls
            .insert(declaration.name.clone(), declaration);
        Ok(())
    }

    /// `impl Name for Type { … }`: every function the trait requires, with
    /// `self` at the type, and the trait's defaults for the rest. One
    /// implementation per trait and type.
    fn implement(&mut self, declaration: &'a ImplDecl) -> Result<(), Diagnostic> {
        let trait_name = declaration.trait_name.as_str();
        let info = self.traits.get(trait_name).cloned().ok_or_else(|| {
            Diagnostic::name(
                declaration.span.clone(),
                format!("no trait named `{trait_name}` is in scope"),
            )
        })?;
        let target = crate::declarations::annotation(&declaration.target)?;
        if let Some(previous) = self.impls.iter().find(|held| {
            held.trait_name == trait_name && held.target.constructor == target.constructor
        }) {
            return Err(Diagnostic::typing(
                declaration.span.clone(),
                format!(
                    "`{trait_name}` is implemented for {target} twice, and the earlier one \
                     is still in force (it begins at byte {})",
                    previous.span.start
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
            let parameters: Vec<TypeRef> = signature
                .parameters
                .iter()
                .map(|parameter| with_self(parameter, &target))
                .collect();
            let result = with_self(&signature.result, &target);
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
            self.impls.push(ImplInfo {
                trait_name: trait_name.to_owned(),
                target: target.clone(),
                span: declaration.span.clone(),
            });
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
        self.impls.push(ImplInfo {
            trait_name: trait_name.to_owned(),
            target: target.clone(),
            span: declaration.span.clone(),
        });
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

    /// The trait function a receiver of a type reaches by name: implemented
    /// for the type, or promised by a trait bound on the type variable.
    /// Two traits offering one name are an ambiguity.
    fn method_for(
        &self,
        receiver: &TypeRef,
        name: &str,
        span: &Range<usize>,
    ) -> Result<Option<(String, MethodSig)>, Diagnostic> {
        let mut found: Vec<(String, MethodSig)> = Vec::new();
        let mut consider = |trait_name: &str, traits: &HashMap<String, TraitInfo>| {
            if found.iter().any(|(held, _)| held == trait_name) {
                return;
            }
            if let Some(info) = traits.get(trait_name)
                && let Some(signature) = info.methods.iter().find(|method| method.name == name)
            {
                found.push((trait_name.to_owned(), signature.clone()));
            }
        };
        for held in &self.impls {
            if receiver.is(&held.target) {
                consider(&held.trait_name, &self.traits);
            }
        }
        for (variable, trait_name) in &self.trait_bounds {
            if variable == receiver {
                consider(trait_name, &self.traits);
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

    /// Whether a type satisfies a trait: implemented for it, or promised by
    /// a bound in force. `Never`, the type of nothing, satisfies every trait.
    fn implements(&self, ty: &TypeRef, trait_name: &str) -> bool {
        *ty == TypeRef::NEVER
            || self
                .impls
                .iter()
                .any(|held| held.trait_name == trait_name && ty.is(&held.target))
            || self
                .trait_bounds
                .iter()
                .any(|(variable, held)| variable == ty && held == trait_name)
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
                // `vault` and the library's bindings over it exist only when
                // a vault is given.
                if name == VAULT
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
                    && let Some((_, signature)) = self.method_for(&left_type, "eq", &span)?
                    && self
                        .impls
                        .iter()
                        .any(|held| held.trait_name == "Eq" && left_type.is(&held.target))
                {
                    let parameter = with_self(&signature.parameters[0], &left_type);
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
                if index_type != TypeRef::INT {
                    return Err(Diagnostic::typing(
                        index.span.clone(),
                        format!("a position is an Int, not {index_type}"),
                    ));
                }
                indexed(&receiver_type).map_err(|message| Diagnostic::typing(span, message))
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
                if let Some(&(combiner, _)) =
                    crate::ast::COMBINERS.iter().find(|(held, _)| held == name)
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
                    let step = self.resolution.step_for(name, &input, positional, &span)?;
                    self.checked.steps.insert(span.clone(), step);
                    self.using(step.name, step.purity == Purity::Effectful);
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
                match &step.kind {
                    Kind::Ident(_) if self.names_function(step) => {
                        return self.application(step, Some(&input_type), &[], span);
                    }
                    Kind::Call { callee, arguments } if self.names_function(callee) => {
                        return self.application(callee, Some(&input_type), arguments, span);
                    }
                    _ => {}
                }
                let reference = step_of(step)?;
                let resolved = self.resolve(&reference, &input_type, &step.span)?;
                self.checked.steps.insert(span.clone(), resolved);
                self.using(resolved.name, resolved.purity == Purity::Effectful);
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
            let Some((_, signature)) = self.method_for(&expected, method, &node)? else {
                return Err(Diagnostic::typing(
                    left.span.clone(),
                    format!(
                        "{}, or two values of a type implementing `{trait_name}`, not {expected}",
                        refused()
                    ),
                ));
            };
            let parameter = with_self(&signature.parameters[0], &expected);
            let actual = self.expecting(right, Some(&parameter))?;
            if !actual.is(&parameter) {
                return Err(Diagnostic::typing(
                    right.span.clone(),
                    format!("`{trait_name}` for {expected} takes {parameter}, not {actual}"),
                ));
            }
            expected = with_self(&signature.result, &expected);
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

    /// Dispatch a pipeline step to the extension that provides it.
    fn step(
        &mut self,
        step: &'static Operation,
        input: &TypeRef,
        input_expr: Option<&Expr>,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        // `filter` over one optional value keeps it or makes it absent, and
        // a narrowing predicate narrows it: `Option<Doc> -> Option<Card>`.
        if step.name == "filter"
            && input.constructor.0 == "Option"
            && input.present().iterated().is_none()
        {
            let held = TypeRef::list(input.present().clone());
            let (result, call) = step
                .spec
                .check(step.name, self, &held, input_expr, arguments, &span)?;
            if let Some(check) = step.check {
                check(self, &call, span)?;
            }
            let element = crate::extensions::collections::narrowing(arguments)
                .and_then(|name| crate::types::named(name, &[]))
                .or_else(|| result.element())
                .unwrap_or_else(|| input.present().clone());
            return Ok(TypeRef::optional(element));
        }
        // `map` over a result maps its success: `Result<T, E> -> Result<U, E>`.
        if step.name == "map"
            && let Some((value, error)) = input.as_result()
        {
            let (value, error) = (value.clone(), error.clone());
            let by = crate::extensions::named_or_first(arguments, "by")
                .ok_or_else(|| Diagnostic::typing(span.clone(), "`map` needs a function"))?;
            if arguments.len() != 1 {
                return Err(Diagnostic::typing(span, "`map` takes one function"));
            }
            let produced = self.lambda_parameters(by, vec![value])?;
            return Ok(TypeRef::result(produced, error));
        }
        // `map` over one optional value maps what is present and keeps
        // absence: `Option<T> -> Option<U>`.
        if step.name == "map"
            && input.constructor.0 == "Option"
            && input.present().iterated().is_none()
            && *input.present() != TypeRef::DATA
        {
            let by = crate::extensions::named_or_first(arguments, "by")
                .ok_or_else(|| Diagnostic::typing(span.clone(), "`map` needs a function"))?;
            if arguments.len() != 1 {
                return Err(Diagnostic::typing(span, "`map` takes one function"));
            }
            let produced = self.lambda_parameters(by, vec![input.present().clone()])?;
            return Ok(TypeRef::optional(produced.present().clone()));
        }
        // A step applied to absence is a step applied to nothing, not a
        // failure: the reference that produced the absence was permitted.
        let input = if step.takes_raw_input() {
            input.clone()
        } else {
            input.present().clone()
        };
        let (result, call) = step
            .spec
            .check(step.name, self, &input, input_expr, arguments, &span)?;
        if let Some(check) = step.check {
            check(self, &call, span)?;
        }
        // A predicate that tests the element's type narrows the result to it.
        if step.name == "filter"
            && let Some(narrowed) = crate::extensions::collections::narrowing(arguments)
                .and_then(|name| crate::types::named(name, &[]))
        {
            return Ok(input.image(narrowed));
        }
        Ok(result)
    }
}

impl CheckCx for Checker<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn warn(&mut self, warning: crate::warnings::Warning) {
        self.checked.warnings.push(warning);
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
        // A constructor passed where a function is: `map(Set)` builds a set
        // from each value it is given.
        if let Kind::Ident(name) = &argument.value.kind
            && let Some(constructor) = crate::constructors::named(name)
            && !self.scope.contains_key(name)
        {
            let span = argument.value.span.clone();
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
            let step = self.resolution.step_for(name, parameter, 0, &span)?;
            self.using(step.name, step.purity == Purity::Effectful);
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
            if !wanted.is(subject) && !subject.is(&wanted) {
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
