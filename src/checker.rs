//! Static checking: what a program's type is, before anything is evaluated.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::builtins;
use crate::diagnostics::Diagnostic;
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
                        format!("`{name}` is a stage and needs an input: write `… | {name}`"),
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
                    return Err(Diagnostic::typing(span, "only a named stage can be called"));
                };
                if builtins::exists(name) {
                    return Err(Diagnostic::typing(
                        span,
                        format!("`{name}` is a stage and needs an input: write `… | {name}(…)`"),
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
            Kind::Pipe(input, stage) => {
                let input_type = self.expression(input)?;
                let (name, arguments) = stage_of(stage)?;
                self.stage(name, &input_type, arguments, stage.span.clone())
            }
        }
    }

    /// Check a lambda argument with its parameter bound to an element type.
    fn lambda(&mut self, argument: &Arg, parameter: Type) -> Result<Type, Diagnostic> {
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

    fn stage(
        &mut self,
        name: &str,
        input: &Type,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Type, Diagnostic> {
        // A stage applied to absence is a stage applied to nothing, not a
        // failure: the reference that produced the absence was permitted.
        let input = match input {
            Type::Option(inner) => inner.as_ref(),
            other => other,
        };
        let collection = |input: &Type| -> Result<Type, Diagnostic> {
            input.element().ok_or_else(|| {
                Diagnostic::typing(
                    span.clone(),
                    format!("`{name}` needs a collection, not {input}"),
                )
            })
        };

        match name {
            "count" => {
                collection(input)?;
                Ok(Type::Int)
            }
            "sort" => {
                let element = collection(input)?;
                match named_or_first(arguments, "by") {
                    Some(argument) => {
                        let key = self.lambda(argument, element.clone())?;
                        if !key.is_orderable() {
                            return Err(Diagnostic::typing(
                                argument.value.span.clone(),
                                format!("a sort key must be orderable, and {key} is not"),
                            ));
                        }
                    }
                    None => require_orderable(&element, name, &span)?,
                }
                Ok(Type::Seq(Box::new(element)))
            }
            "take" => {
                let element = collection(input)?;
                let argument = named_or_first(arguments, "n").ok_or_else(|| {
                    Diagnostic::typing(span.clone(), "`take` needs a count: `take(5)`")
                })?;
                let count = self.expression(&argument.value)?;
                if count != Type::Int {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("`take` needs an Int count, not {count}"),
                    ));
                }
                // A prefix must be reproducible, which needs the elements to
                // carry an order of their own rather than the order they were
                // discovered in.
                require_orderable(&element, name, &span)?;
                Ok(match input {
                    Type::Ranking(_) => input.clone(),
                    _ => Type::Seq(Box::new(element)),
                })
            }
            "filter" => {
                let element = collection(input)?;
                let argument = named_or_first(arguments, "by").ok_or_else(|| {
                    Diagnostic::typing(span.clone(), "`filter` needs a predicate: `filter(c => …)`")
                })?;
                let result = self.lambda(argument, element.clone())?;
                if result != Type::Bool {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("a filter predicate returns Bool, not {result}"),
                    ));
                }
                Ok(input.with_element(element))
            }
            "map" => {
                let element = collection(input)?;
                let argument = named_or_first(arguments, "by").ok_or_else(|| {
                    Diagnostic::typing(span.clone(), "`map` needs a function: `map(c => …)`")
                })?;
                // Mapping a set does not decide whether equal outputs collapse,
                // so the result is a sequence and the question stays open.
                Ok(Type::Seq(Box::new(self.lambda(argument, element)?)))
            }
            "typed" => {
                let element = collection(input)?;
                let argument = named_or_first(arguments, "as").ok_or_else(|| {
                    Diagnostic::typing(span.clone(), "`typed` needs a type: `typed(RelationCard)`")
                })?;
                let Kind::Ident(type_name) = &argument.value.kind else {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        "`typed` takes a type name",
                    ));
                };
                let wanted = types::named(type_name, &[]).ok_or_else(|| {
                    Diagnostic::name(
                        argument.value.span.clone(),
                        format!("unknown type `{type_name}`"),
                    )
                })?;
                if !wanted.is(&element) && !element.is(&wanted) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("no {element} can be a {wanted}, so this selects nothing"),
                    ));
                }
                Ok(input.with_element(wanted))
            }
            "uplinks" | "downlinks" => {
                if name == "downlinks" && !self.vault.is_present() {
                    return Err(Diagnostic::name(
                        span,
                        "`downlinks` needs a vault: a document cannot know what points at it",
                    ));
                }
                let subject = input.element().unwrap_or_else(|| input.clone());
                if !subject.is(&Type::Doc) && !matches!(subject, Type::Hit(_)) {
                    return Err(Diagnostic::typing(
                        span,
                        format!("`{name}` needs a card or cards, not {input}"),
                    ));
                }
                Ok(Type::Seq(Box::new(Type::Edge)))
            }
            "semantic" => {
                let element = collection(input)?;
                if !element.is(&Type::Doc) {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!("`semantic` ranks documents, not {element}"),
                    ));
                }
                let argument = named_or_first(arguments, "query").ok_or_else(|| {
                    Diagnostic::typing(span.clone(), "`semantic` needs a query: `semantic(\"…\")`")
                })?;
                let query = self.expression(&argument.value)?;
                if query != Type::Str {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!("a query is text, not {query}"),
                    ));
                }
                Ok(Type::Ranking(Box::new(Type::Card)))
            }
            "expand" => {
                if let Some(argument) = named_or_first(arguments, "depth") {
                    let depth = self.expression(&argument.value)?;
                    if depth != Type::Int {
                        return Err(Diagnostic::typing(
                            argument.value.span.clone(),
                            format!("`depth` is an Int, not {depth}"),
                        ));
                    }
                }
                if let Some(argument) = named(arguments, "direction") {
                    let direction = self.expression(&argument.value)?;
                    if direction != Type::Str {
                        return Err(Diagnostic::typing(
                            argument.value.span.clone(),
                            format!("`direction` is text, not {direction}"),
                        ));
                    }
                }
                if !self.vault.is_present() {
                    return Err(Diagnostic::name(span, "`expand` needs a vault to traverse"));
                }
                graph_input(input, name, &span)?;
                Ok(Type::Graph)
            }
            "graph" => {
                graph_input(input, name, &span)?;
                Ok(Type::Graph)
            }
            "table" | "json" | "text" => Ok(Type::Presentation),
            other => {
                let hint = builtins::nearest(other)
                    .map(|near| format!("; did you mean `{near}`?"))
                    .unwrap_or_default();
                Err(Diagnostic::name(
                    span,
                    format!("`{other}` is not a stage{hint}"),
                ))
            }
        }
    }
}

fn graph_input(input: &Type, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
    if *input == Type::Graph || input.is(&Type::Doc) {
        // A single card projects to the graph of one node, which is what a
        // traversal from one card needs as its seed.
        return Ok(());
    }
    let element = input.element().ok_or_else(|| {
        Diagnostic::typing(
            span.clone(),
            format!("`{name}` needs a collection of cards, not {input}"),
        )
    })?;
    let subject = match &element {
        Type::Hit(inner) => inner.as_ref().clone(),
        other => other.clone(),
    };
    if subject.is(&Type::Doc) {
        Ok(())
    } else {
        Err(Diagnostic::typing(
            span.clone(),
            format!("`{name}` needs cards, not {element}"),
        ))
    }
}

fn require_orderable(element: &Type, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
    if element.is_orderable() {
        return Ok(());
    }
    Err(Diagnostic::typing(
        span.clone(),
        format!(
            "`{name}` needs elements with an order of their own, and {element} has none; \
             sort with an explicit key first"
        ),
    ))
}

fn named<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some(name))
}

fn named_or_first<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    named(arguments, name).or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
}

/// The name and arguments of a pipeline stage.
pub(crate) fn stage_of(stage: &Expr) -> Result<(&str, &[Arg]), Diagnostic> {
    match &stage.kind {
        Kind::Ident(name) => Ok((name, &[])),
        Kind::Call { callee, arguments } => match &callee.kind {
            Kind::Ident(name) => Ok((name, arguments)),
            _ => Err(Diagnostic::typing(
                stage.span.clone(),
                "a pipeline stage is a name or a call",
            )),
        },
        _ => Err(Diagnostic::typing(
            stage.span.clone(),
            "a pipeline stage is a name or a call",
        )),
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
