//! Static checking: what a program's type is, before anything is evaluated.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::builtins;
use crate::diagnostics::Diagnostic;
use crate::extensions::{CheckCx, Resolution, Step};
use crate::types::{self, Type};
use crate::vault::Vault;
use std::collections::HashMap;
use std::ops::Range;

/// Infer the type of a whole program against a vault.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub(crate) fn check(program: &Program, vault: &Vault) -> Result<Type, Diagnostic> {
    Checker {
        vault,
        scope: HashMap::new(),
        resolution: Resolution::new(true),
    }
    .program(program)
}

/// Check every statement, continuing past one that fails.
///
/// A statement that does not check still binds its name, at the open tree
/// type, so the statements after it are reported on their own faults rather
/// than on a cascade from this one.
pub(crate) fn check_collecting(program: &Program, vault: &Vault) -> (Type, Vec<Diagnostic>) {
    let mut checker = Checker {
        vault,
        scope: HashMap::new(),
        resolution: Resolution::new(true),
    };
    let mut found = Vec::new();
    let mut last = Type::Unit;
    for statement in &program.statements {
        match checker.statement(statement) {
            Ok(inferred) => last = inferred,
            Err(diagnostic) => {
                if let Stmt::Bind { name, .. } = statement {
                    checker.scope.insert(name.clone(), Type::Data);
                }
                found.push(diagnostic);
                last = Type::Unit;
            }
        }
    }
    (last, found)
}

struct Checker<'a> {
    vault: &'a Vault,
    scope: HashMap<String, Type>,
    /// Which extensions this program has imported.
    resolution: Resolution,
}

impl Checker<'_> {
    fn program(&mut self, program: &Program) -> Result<Type, Diagnostic> {
        let mut last = Type::Unit;
        for statement in &program.statements {
            last = self.statement(statement)?;
        }
        Ok(last)
    }

    fn statement(&mut self, statement: &Stmt) -> Result<Type, Diagnostic> {
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
                        let arguments = annotation
                            .arguments
                            .iter()
                            .map(|argument| {
                                types::named(&argument.name, &[]).ok_or_else(|| {
                                    Diagnostic::name(
                                        argument.span.clone(),
                                        format!("unknown type `{}`", argument.name),
                                    )
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let declared =
                            types::named(&annotation.name, &arguments).ok_or_else(|| {
                                Diagnostic::name(
                                    annotation.span.clone(),
                                    format!("unknown type `{}`", annotation.name),
                                )
                            })?;
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
                    Type::Unit
                }
                Stmt::Import { name, span } => {
                    self.resolution.import(name, span)?;
                    Type::Unit
                }
                Stmt::Expr(expression) => self.expression(expression)?,
            }
        })
    }

    fn expression(&mut self, expression: &Expr) -> Result<Type, Diagnostic> {
        let span = expression.span.clone();
        match &expression.kind {
            Kind::Int(_) => Ok(Type::Int),
            Kind::Float(_) => Ok(Type::Float),
            Kind::Bool(_) => Ok(Type::Bool),
            Kind::Str(_) => Ok(Type::Str),
            Kind::Data(entries) => {
                for (_, value) in entries {
                    self.expression(value)?;
                }
                Ok(Type::Data)
            }
            // A reference may not resolve — a forward link to a document
            // nobody has written is ordinary — so it is optional by type.
            Kind::DocRef(_) => Ok(Type::Option(Box::new(Type::Card))),
            Kind::Cards => {
                if self.vault.is_present() {
                    Ok(Type::Set(Box::new(Type::Card)))
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
            Kind::Add(left, right) => {
                // Addition is homogeneous: there is no implicit widening.
                let expected = self.expression(left)?;
                if !matches!(expected, Type::Int | Type::Float) {
                    return Err(Diagnostic::typing(
                        left.span.clone(),
                        "addition requires two Int or two Float operands",
                    ));
                }
                let found = self.expression(right)?;
                if found != expected {
                    return Err(Diagnostic::typing(
                        right.span.clone(),
                        "addition requires two Int or two Float operands",
                    ));
                }
                Ok(expected)
            }
            Kind::Compare { left, right, .. } => {
                let left_type = self.expression(left)?;
                let right_type = self.expression(right)?;
                // An open tree's leaf may be compared with a literal: that is
                // what asking a header a question looks like.
                let comparable =
                    left_type == right_type || left_type == Type::Data || right_type == Type::Data;
                if !comparable {
                    return Err(Diagnostic::typing(
                        span,
                        format!("{left_type} and {right_type} are never equal"),
                    ));
                }
                Ok(Type::Bool)
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
                    let resolved = match &end_type {
                        Type::Option(inner) => inner.as_ref().clone(),
                        other => other.clone(),
                    };
                    if !resolved.is(&Type::Doc) && resolved != Type::Str {
                        return Err(Diagnostic::typing(
                            end.span.clone(),
                            format!("a link endpoint is a document reference, not {end_type}"),
                        ));
                    }
                }
                Ok(Type::Edge)
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
                if builtins::exists(name) {
                    return Err(Diagnostic::typing(
                        span,
                        format!(
                            "`{name}` is a pipeline step and needs an input: write `… | {name}(…)`"
                        ),
                    ));
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

    /// Check a lambda argument with its parameter bound to an element type.
    fn lambda_type(&mut self, argument: &Arg, parameter: Type) -> Result<Type, Diagnostic> {
        let Kind::Lambda {
            parameter: name,
            body,
        } = &argument.value.kind
        else {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "expected a function, such as `c => c.title`",
            ));
        };
        let shadowed = self.scope.insert(name.clone(), parameter);
        let result = self.expression(body);
        match shadowed {
            Some(previous) => self.scope.insert(name.clone(), previous),
            None => self.scope.remove(name),
        };
        result
    }

    /// Which step a written reference names, given what is imported.
    ///
    /// A binding shadows the namespace and not the step: while `semantic` is
    /// bound, `semantic.rank` reads a field of that value, and the step stays
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
        input: &Type,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Type, Diagnostic> {
        // A step applied to absence is a step applied to nothing, not a
        // failure: the reference that produced the absence was permitted.
        let input = match input {
            Type::Option(inner) => inner.as_ref().clone(),
            other => other.clone(),
        };
        (step.check)(self, &input, arguments, span)
    }
}

impl CheckCx for Checker<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn infer(&mut self, expression: &Expr) -> Result<Type, Diagnostic> {
        self.expression(expression)
    }

    fn lambda(&mut self, argument: &Arg, parameter: Type) -> Result<Type, Diagnostic> {
        self.lambda_type(argument, parameter)
    }
}

/// How a pipeline step was written: bare, or qualified by its extension.
pub(crate) enum StepRef<'a> {
    /// `| take(5)` — resolved against everything the program imported.
    Bare { name: &'a str, arguments: &'a [Arg] },
    /// `| semantic.semantic("…")` — always available for an imported
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
pub(crate) fn field_type(receiver: &Type, field: &str) -> Option<Type> {
    match receiver {
        Type::Option(inner) => field_type(inner, field).map(|inner| Type::Option(Box::new(inner))),
        Type::Data => Some(Type::Data),
        Type::Doc | Type::Card | Type::ConceptCard | Type::RelationCard => {
            let card = receiver.is_card();
            match field {
                "name" | "title" | "path" | "format" | "body" => Some(Type::Str),
                "header" => Some(Type::Data),
                "metadata" if card => Some(Type::Data),
                "kind" if card => Some(Type::Str),
                _ => None,
            }
        }
        Type::Edge => match field {
            "source" | "target" => Some(Type::Str),
            "data" => Some(Type::Data),
            _ => None,
        },
        Type::Hit(element) => match field {
            "card" => Some(element.as_ref().clone()),
            "score" => Some(Type::Float),
            "query" => Some(Type::Str),
            "retrieval" => Some(Type::Data),
            _ => None,
        },
        Type::Ranking(element) => match field {
            "hits" => Some(Type::Seq(Box::new(Type::Hit(element.clone())))),
            "query" => Some(Type::Str),
            "retrieval" => Some(Type::Data),
            _ => None,
        },
        Type::Graph => match field {
            "nodes" => Some(Type::Seq(Box::new(Type::Str))),
            "edges" => Some(Type::Seq(Box::new(Type::Edge))),
            _ => None,
        },
        _ => None,
    }
}
