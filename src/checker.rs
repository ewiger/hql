//! Static checking: what a program's type is, before anything is evaluated.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::builtins;
use crate::declarations::Declarations;
use crate::diagnostics::Diagnostic;
use crate::extensions::{CheckCx, Resolution, Step};
use crate::types::TypeRef;
use crate::vault::Vault;
use std::collections::HashMap;
use std::ops::Range;

/// Infer the type of a whole program against a vault.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub(crate) fn check(program: &Program, vault: &Vault) -> Result<TypeRef, Diagnostic> {
    Checker {
        vault,
        scope: HashMap::new(),
        resolution: Resolution::new(&vault.extensions)?,
        declarations: Declarations::default(),
    }
    .program(program)
}

/// Check every statement, continuing past one that fails.
///
/// A statement that does not check still binds its name, at the open tree
/// type, so the statements after it are reported on their own faults rather
/// than on a cascade from this one.
pub(crate) fn check_collecting(program: &Program, vault: &Vault) -> (TypeRef, Vec<Diagnostic>) {
    let resolution = match Resolution::new(&vault.extensions) {
        Ok(resolution) => resolution,
        // A vault that cannot resolve its own configuration has nothing to
        // say about the program: every statement would fail for one reason.
        Err(diagnostic) => return (TypeRef::UNIT, vec![diagnostic]),
    };
    let mut checker = Checker {
        vault,
        scope: HashMap::new(),
        resolution,
        declarations: Declarations::default(),
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
    (last, found)
}

struct Checker<'a> {
    vault: &'a Vault,
    scope: HashMap<String, TypeRef>,
    /// Which extensions this program has imported.
    resolution: Resolution,
    /// What this program has declared.
    declarations: Declarations,
}

impl Checker<'_> {
    fn program(&mut self, program: &Program) -> Result<TypeRef, Diagnostic> {
        let mut last = TypeRef::UNIT;
        for statement in &program.statements {
            last = self.statement(statement)?;
        }
        Ok(last)
    }

    fn statement(&mut self, statement: &Stmt) -> Result<TypeRef, Diagnostic> {
        Ok({
            match statement {
                Stmt::Bind {
                    name,
                    annotation,
                    value,
                    ..
                } => {
                    let inferred = self.expression(value)?;
                    if let Some(annotation) = annotation {
                        let declared = crate::declarations::annotation(annotation)?;
                        if !inferred.is(&declared) {
                            return Err(Diagnostic::typing(
                                value.span.clone(),
                                format!(
                                    "`{name}` is declared {declared} but its value is {inferred}"
                                ),
                            ));
                        }
                        self.scope.insert(name.clone(), declared);
                    } else {
                        self.scope.insert(name.clone(), inferred);
                    }
                    TypeRef::UNIT
                }
                Stmt::Import { name, span } => {
                    self.resolution.import(name, span)?;
                    TypeRef::UNIT
                }
                // A declaration is checked and yields nothing to show. It
                // introduces no value: constructing one is separate work.
                Stmt::Type(declaration) => {
                    self.declarations.declare(declaration)?;
                    TypeRef::UNIT
                }
                Stmt::Expr(expression) => self.expression(expression)?,
            }
        })
    }

    fn expression(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic> {
        let span = expression.span.clone();
        match &expression.kind {
            Kind::List(values) | Kind::Set(values) => {
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
            Kind::Cards => {
                if self.vault.is_present() {
                    Ok(TypeRef::set(TypeRef::CARD))
                } else {
                    Err(Diagnostic::name(
                        span,
                        "`cards` needs a vault: pass --vault <directory>",
                    ))
                }
            }
            Kind::Ident(name) => self.scope.get(name).cloned().ok_or_else(|| {
                if builtins::exists(name) {
                    Diagnostic::typing(
                        span,
                        format!(
                            "`{name}` is a pipeline step and needs an input: write `… | {name}`"
                        ),
                    )
                } else {
                    let hint = builtins::nearest(name)
                        .map(|near| format!("; did you mean `{near}`?"))
                        .unwrap_or_default();
                    Diagnostic::name(span, format!("`{name}` is not bound{hint}"))
                }
            }),
            Kind::Add(..) => self.addition(expression),
            Kind::Compare { left, right, .. } => {
                let left_type = self.expression(left)?;
                let right_type = self.expression(right)?;
                // An open tree's leaf may be compared with a literal: that is
                // what asking a header a question looks like.
                let comparable = left_type == right_type
                    || left_type == TypeRef::DATA
                    || right_type == TypeRef::DATA;
                if !comparable {
                    return Err(Diagnostic::typing(
                        span,
                        format!("{left_type} and {right_type} are never equal"),
                    ));
                }
                Ok(TypeRef::BOOL)
            }
            Kind::Field(receiver, field) => {
                let receiver_type = self.expression(receiver)?;
                field_type(&receiver_type, field).ok_or_else(|| {
                    Diagnostic::name(span, format!("{receiver_type} has no field `{field}`"))
                })
            }
            Kind::Link { source, target, .. } => {
                for end in [source, target] {
                    let end_type = self.expression(end)?;
                    let resolved = end_type.present().clone();
                    if !resolved.is(&TypeRef::DOC) && resolved != TypeRef::STR {
                        return Err(Diagnostic::typing(
                            end.span.clone(),
                            format!("a link endpoint is a document reference, not {end_type}"),
                        ));
                    }
                }
                Ok(TypeRef::EDGE)
            }
            Kind::Lambda { .. } => Err(Diagnostic::typing(
                span,
                "a lambda may only be written as an argument, such as `filter(c => …)`",
            )),
            Kind::Call { callee, arguments } => {
                let Kind::Ident(name) = &callee.kind else {
                    return Err(Diagnostic::typing(
                        span,
                        "only a named pipeline step can be called",
                    ));
                };
                if let Some(constructor) = crate::constructors::named(name) {
                    return crate::constructors::check(self, constructor, arguments, None, span);
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
                    let input = self.expression(&input.value)?;
                    let step = self.resolution.step(name, &span)?;
                    return self.step(step, &input, arguments, span);
                }
                let _ = arguments;
                let hint = builtins::nearest(name)
                    .map(|near| format!("; did you mean `{near}`?"))
                    .unwrap_or_default();
                Err(Diagnostic::name(
                    span,
                    format!("`{name}` is not bound{hint}"),
                ))
            }
            Kind::Pipe(input, step) => {
                let input_type = self.expression(input)?;
                let reference = step_of(step)?;
                let resolved = self.resolve(&reference, &step.span)?;
                self.step(
                    resolved,
                    &input_type,
                    reference.arguments(),
                    step.span.clone(),
                )
            }
        }
    }

    /// Check a left-associated chain without consuming stack per addition.
    fn addition(&mut self, mut expression: &Expr) -> Result<TypeRef, Diagnostic> {
        let mut operands = Vec::new();
        while let Kind::Add(left, right) = &expression.kind {
            operands.push((left.as_ref(), right.as_ref()));
            expression = left;
        }
        let expected = self.expression(expression)?;
        for (left, right) in operands.into_iter().rev() {
            // Addition is homogeneous: there is no implicit widening.
            if !matches!(expected.constructor.0, "Int" | "Float") {
                return Err(Diagnostic::typing(
                    left.span.clone(),
                    "addition requires two Int or two Float operands",
                ));
            }
            if self.expression(right)? != expected {
                return Err(Diagnostic::typing(
                    right.span.clone(),
                    "addition requires two Int or two Float operands",
                ));
            }
        }
        Ok(expected)
    }

    /// Check a lambda argument with its parameter bound to an element type.
    fn lambda_type(&mut self, argument: &Arg, types: Vec<TypeRef>) -> Result<TypeRef, Diagnostic> {
        let Kind::Lambda { parameters, body } = &argument.value.kind else {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "expected a lambda",
            ));
        };
        if parameters.len() != types.len() {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                format!("expected {} lambda parameters", types.len()),
            ));
        }
        let previous = parameters
            .iter()
            .zip(types)
            .map(|(name, ty)| (name, self.scope.insert(name.clone(), ty)))
            .collect::<Vec<_>>();
        let result = self.expression(body);
        for (name, value) in previous {
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
        span: &Range<usize>,
    ) -> Result<&'static Step, Diagnostic> {
        match reference {
            StepRef::Bare { name, .. } => self.resolution.step(name, span),
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

    /// Dispatch a pipeline step to the extension that provides it.
    fn step(
        &mut self,
        step: &'static Step,
        input: &TypeRef,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        // A step applied to absence is a step applied to nothing, not a
        // failure: the reference that produced the absence was permitted.
        let input = input.present().clone();
        (step.check)(self, &input, arguments, span)
    }
}

impl CheckCx for Checker<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn infer(&mut self, expression: &Expr) -> Result<TypeRef, Diagnostic> {
        self.expression(expression)
    }

    fn lambda(&mut self, argument: &Arg, parameter: TypeRef) -> Result<TypeRef, Diagnostic> {
        self.lambda_type(argument, vec![parameter])
    }
    fn comparator(&mut self, argument: &Arg, parameter: TypeRef) -> Result<TypeRef, Diagnostic> {
        self.lambda_type(argument, vec![parameter.clone(), parameter])
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

/// The type of a field on a value of a type.
pub(crate) fn field_type(receiver: &TypeRef, field: &str) -> Option<TypeRef> {
    if field == "keys" && crate::types::MapKind::of(receiver.constructor).is_some() {
        return crate::types::collections::keys_type(
            crate::types::builtin::system().ok()?,
            receiver,
        )
        .ok();
    }
    match receiver.constructor.0 {
        "Option" => field_type(receiver.args.first()?, field).map(TypeRef::optional),
        "Data" => Some(TypeRef::DATA),
        "Doc" | "Card" | "ConceptCard" | "RelationCard" => {
            let card = receiver.is_card();
            match field {
                "name" | "title" | "path" | "format" | "body" => Some(TypeRef::STR),
                "header" => Some(TypeRef::DATA),
                "metadata" if card => Some(TypeRef::DATA),
                "kind" if card => Some(TypeRef::STR),
                _ => None,
            }
        }
        "Edge" => match field {
            "source" | "target" => Some(TypeRef::STR),
            "data" => Some(TypeRef::DATA),
            _ => None,
        },
        "Hit" => match field {
            "card" => Some(receiver.args.first()?.clone()),
            "score" => Some(TypeRef::FLOAT),
            "query" => Some(TypeRef::STR),
            "retrieval" => Some(TypeRef::DATA),
            _ => None,
        },
        "Ranking" => match field {
            "hits" => Some(TypeRef::list(TypeRef::hit(receiver.args.first()?.clone()))),
            "query" => Some(TypeRef::STR),
            "retrieval" => Some(TypeRef::DATA),
            _ => None,
        },
        "Graph" => match field {
            "nodes" => Some(TypeRef::list(TypeRef::STR)),
            "edges" => Some(TypeRef::list(TypeRef::EDGE)),
            _ => None,
        },
        _ => None,
    }
}
