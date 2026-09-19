//! Structural step contracts shared by elaboration and the builtin catalogue.

use super::CheckCx;
use crate::ast::{Arg, Kind};
use crate::diagnostics::Diagnostic;
use crate::types::{TypeConstructor, TypeRef};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

/// A concrete, generic, or structural type in a step declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePattern {
    /// Any value, without a type constraint.
    Any,
    /// A type or any of its subtypes.
    Type(TypeConstructor),
    /// A generic parameter bound by the input or an argument.
    Var(&'static str),
    /// A constructor applied to structural arguments.
    Apply(TypeConstructor, &'static [TypePattern]),
    /// Any collection, including retrieval carriers.
    Collection(&'static TypePattern),
    /// Any one of the alternatives.
    OneOf(&'static [TypePattern]),
}

/// A generic parameter and its optional subtype bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeParameter {
    /// Parameter name used in the structural patterns.
    pub name: &'static str,
    /// Required supertype, when constrained.
    pub bound: Option<TypeConstructor>,
}

/// How an argument is elaborated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentKind {
    /// An ordinary value checked against a pattern.
    Value(TypePattern),
    /// A lambda checked in a scope containing these parameters.
    Lambda {
        parameters: &'static [TypePattern],
        result: TypePattern,
    },
    /// An orderable key selector or a two-parameter comparator.
    Ordering(TypePattern),
    /// A type name bound to a generic parameter.
    TypeName(&'static str),
}

/// An accepted argument, including how it may be supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgumentSpec {
    /// Name shown in help and accepted for named arguments.
    pub name: &'static str,
    /// Its value or dependent argument contract.
    pub kind: ArgumentKind,
    /// Whether omission is an error.
    pub required: bool,
    /// Whether it can be supplied positionally.
    pub positional: bool,
    /// Whether it can be supplied by name.
    pub named: bool,
}

impl ArgumentSpec {
    /// Declare a required value argument accepted positionally or by name.
    pub const fn value(name: &'static str, ty: TypePattern) -> Self {
        Self {
            name,
            kind: ArgumentKind::Value(ty),
            required: true,
            positional: true,
            named: true,
        }
    }
}

/// The result expression, including operations that retain input structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// Instantiate a declared type pattern.
    Type(TypePattern),
    /// Preserve the input's type.
    Input,
    /// Preserve the input collection with a narrowed element type.
    WithElement(&'static str),
    /// Produce a list, retaining a ranking's provenance when the input is ranked.
    Prefix(&'static str),
}

/// The machine-readable signature used by both checking and help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepSpec {
    /// Generic parameters introduced by this signature.
    pub parameters: &'static [TypeParameter],
    /// The input before the pipe.
    pub input: TypePattern,
    /// Arguments following the step name.
    pub arguments: &'static [ArgumentSpec],
    /// The result after generic substitution.
    pub output: Output,
}

/// An incoherent extension declaration, detected before checking a program.
#[derive(Debug, thiserror::Error)]
#[error("invalid step signature: {0}")]
pub struct SpecError(String);

/// Facts available to a dependent checking hook, after signature checking.
pub(crate) struct CheckedCall<'a> {
    pub input: &'a TypeRef,
    pub arguments: &'a [Arg],
    pub bindings: BTreeMap<&'static str, TypeRef>,
}

impl TypePattern {
    fn accept(self, actual: &TypeRef, bindings: &mut BTreeMap<&'static str, TypeRef>) -> bool {
        match self {
            Self::Any => true,
            Self::Type(constructor) => actual.is(&TypeRef::from(constructor)),
            Self::Var(name) => match bindings.get(name) {
                Some(expected) => *expected == TypeRef::NEVER || actual.is(expected),
                None => {
                    bindings.insert(name, actual.clone());
                    true
                }
            },
            Self::Collection(element) => actual
                .element()
                .is_some_and(|actual| element.accept(&actual, bindings)),
            Self::Apply(constructor, patterns) => {
                let shape = constructor.apply(actual.args.clone());
                actual.is(&shape)
                    && patterns.len() == actual.args.len()
                    && patterns
                        .iter()
                        .zip(&actual.args)
                        .all(|(pattern, actual)| pattern.accept(actual, bindings))
            }
            Self::OneOf(patterns) => patterns.iter().any(|pattern| {
                let mut trial = bindings.clone();
                if pattern.accept(actual, &mut trial) {
                    *bindings = trial;
                    true
                } else {
                    false
                }
            }),
        }
    }

    fn instantiate(
        self,
        bindings: &BTreeMap<&'static str, TypeRef>,
        span: &Range<usize>,
    ) -> Result<TypeRef, Diagnostic> {
        match self {
            Self::Type(constructor) => Ok(TypeRef::from(constructor)),
            Self::Var(name) => bindings.get(name).cloned().ok_or_else(|| {
                Diagnostic::typing(
                    span.clone(),
                    format!("unbound signature parameter `{name}`"),
                )
            }),
            Self::Apply(constructor, args) => Ok(constructor.apply(
                args.iter()
                    .map(|arg| arg.instantiate(bindings, span))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            _ => Err(Diagnostic::typing(
                span.clone(),
                "a result needs a concrete type expression",
            )),
        }
    }
}

impl StepSpec {
    /// Verify that names, generic references, and type constructors are coherent.
    ///
    /// # Errors
    /// Rejects duplicate names, undeclared parameters, and invalid constructors.
    pub fn validate(&self) -> Result<(), SpecError> {
        let system =
            crate::types::builtin::system().map_err(|error| SpecError(error.to_string()))?;
        let mut parameters = std::collections::BTreeSet::new();
        for parameter in self.parameters {
            if !parameters.insert(parameter.name) {
                return Err(SpecError(format!(
                    "duplicate parameter `{}`",
                    parameter.name
                )));
            }
            if let Some(bound) = parameter.bound {
                system
                    .definition(bound)
                    .map_err(|error| SpecError(error.to_string()))?;
            }
        }
        let mut patterns = vec![self.input];
        let mut names = std::collections::BTreeSet::new();
        for argument in self.arguments {
            if !names.insert(argument.name) || (!argument.positional && !argument.named) {
                return Err(SpecError(format!(
                    "duplicate or unreachable argument `{}`",
                    argument.name
                )));
            }
            match argument.kind {
                ArgumentKind::Value(pattern) | ArgumentKind::Ordering(pattern) => {
                    patterns.push(pattern)
                }
                ArgumentKind::TypeName(parameter) => patterns.push(TypePattern::Var(parameter)),
                ArgumentKind::Lambda { parameters, result } => {
                    patterns.extend(parameters);
                    patterns.push(result);
                }
            }
        }
        match self.output {
            Output::Type(pattern) => patterns.push(pattern),
            Output::WithElement(parameter) | Output::Prefix(parameter) => {
                patterns.push(TypePattern::Var(parameter))
            }
            Output::Input => {}
        }
        while let Some(pattern) = patterns.pop() {
            match pattern {
                TypePattern::Var(name) if !parameters.contains(name) => {
                    return Err(SpecError(format!("undeclared parameter `{name}`")));
                }
                TypePattern::Var(_) | TypePattern::Any => {}
                TypePattern::Type(constructor) => {
                    let definition = system
                        .definition(constructor)
                        .map_err(|error| SpecError(error.to_string()))?;
                    if !definition.parameters.is_empty() {
                        return Err(SpecError(format!("{constructor} needs type arguments")));
                    }
                }
                TypePattern::Apply(constructor, arguments) => {
                    let definition = system
                        .definition(constructor)
                        .map_err(|error| SpecError(error.to_string()))?;
                    if definition.parameters.len() != arguments.len() {
                        return Err(SpecError(format!("wrong arity for {constructor}")));
                    }
                    patterns.extend(arguments);
                }
                TypePattern::Collection(element) => patterns.push(*element),
                TypePattern::OneOf(alternatives) => {
                    if alternatives.is_empty() {
                        return Err(SpecError("empty alternatives".into()));
                    }
                    patterns.extend(alternatives);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn check<'a>(
        &self,
        name: &str,
        cx: &mut dyn CheckCx,
        input: &'a TypeRef,
        arguments: &'a [Arg],
        span: &Range<usize>,
    ) -> Result<(TypeRef, CheckedCall<'a>), Diagnostic> {
        let mut bindings = BTreeMap::new();
        if !self.input.accept(input, &mut bindings) {
            return Err(Diagnostic::typing(
                span.clone(),
                format!("`{name}` needs {}, not {input}", self.input),
            ));
        }
        let mut supplied = vec![None; self.arguments.len()];
        let mut positions = self
            .arguments
            .iter()
            .enumerate()
            .filter(|(_, arg)| arg.positional)
            .map(|(index, _)| index);
        for argument in arguments {
            let index = match argument.name.as_deref() {
                Some(written) => self
                    .arguments
                    .iter()
                    .position(|spec| spec.named && spec.name == written),
                None => positions.next(),
            }
            .ok_or_else(|| {
                Diagnostic::typing(
                    argument.value.span.clone(),
                    format!("unexpected argument to `{name}`"),
                )
            })?;
            if supplied[index].replace(argument).is_some() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!(
                        "`{}` is supplied twice to `{name}`",
                        self.arguments[index].name
                    ),
                ));
            }
        }
        for (spec, argument) in self.arguments.iter().zip(supplied) {
            let Some(argument) = argument else {
                if spec.required {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!("`{name}` needs `{}`", spec.name),
                    ));
                }
                continue;
            };
            let argument_span = &argument.value.span;
            let (actual, pattern) = match spec.kind {
                ArgumentKind::Value(pattern) => (cx.infer(&argument.value)?, pattern),
                ArgumentKind::TypeName(parameter) => {
                    (cx.type_argument(argument)?, TypePattern::Var(parameter))
                }
                ArgumentKind::Lambda { parameters, result } => {
                    let parameters = parameters
                        .iter()
                        .map(|parameter| parameter.instantiate(&bindings, argument_span))
                        .collect::<Result<Vec<_>, _>>()?;
                    (cx.lambda_parameters(argument, parameters)?, result)
                }
                ArgumentKind::Ordering(element) => {
                    let element = element.instantiate(&bindings, argument_span)?;
                    let comparator = matches!(&argument.value.kind, Kind::Lambda { parameters, .. } if parameters.len() == 2);
                    let actual = cx.lambda_parameters(
                        argument,
                        if comparator {
                            vec![element.clone(), element]
                        } else {
                            vec![element]
                        },
                    )?;
                    let expected = if comparator {
                        TypeRef::ORDERING
                    } else {
                        TypeRef::ORDERABLE
                    };
                    if !actual.is(&expected) {
                        return Err(Diagnostic::typing(
                            argument_span.clone(),
                            format!(
                                "a sort {} must return {expected}, not {actual}",
                                if comparator { "comparison" } else { "key" }
                            ),
                        ));
                    }
                    continue;
                }
            };
            if !pattern.accept(&actual, &mut bindings) {
                return Err(Diagnostic::typing(
                    argument_span.clone(),
                    format!(
                        "`{name}` argument `{}` needs {pattern}, not {actual}",
                        spec.name
                    ),
                ));
            }
        }
        for parameter in self.parameters {
            if let Some(bound) = parameter.bound {
                let actual = TypePattern::Var(parameter.name).instantiate(&bindings, span)?;
                if !actual.is(&TypeRef::from(bound)) {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!(
                            "`{name}` requires {} : {bound}, not {actual}",
                            parameter.name
                        ),
                    ));
                }
            }
        }
        let result = match self.output {
            Output::Type(pattern) => pattern.instantiate(&bindings, span)?,
            Output::Input => input.clone(),
            Output::WithElement(parameter) => {
                input.with_element(TypePattern::Var(parameter).instantiate(&bindings, span)?)
            }
            Output::Prefix(parameter) => {
                if input.constructor.0 == "Ranking" {
                    input.clone()
                } else {
                    TypeRef::list(TypePattern::Var(parameter).instantiate(&bindings, span)?)
                }
            }
        };
        Ok((
            result,
            CheckedCall {
                input,
                arguments,
                bindings,
            },
        ))
    }

    /// Render the same contract the checker consumes.
    #[must_use]
    pub fn signature(&self, name: &str) -> String {
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| match parameter.bound {
                Some(bound) => format!("{} : {bound}", parameter.name),
                None => parameter.name.to_owned(),
            })
            .collect::<Vec<_>>();
        let generics = if parameters.is_empty() {
            String::new()
        } else {
            format!("<{}>", parameters.join(", "))
        };
        let arguments = self
            .arguments
            .iter()
            .map(|argument| {
                let kind = match argument.kind {
                    ArgumentKind::Value(pattern) => pattern.to_string(),
                    ArgumentKind::TypeName(parameter) => format!("Type<{parameter}>"),
                    ArgumentKind::Lambda { parameters, result } => format!(
                        "({}) -> {result}",
                        parameters
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    ArgumentKind::Ordering(element) => {
                        format!("({element} -> Orderable) | (({element}, {element}) -> Ordering)")
                    }
                };
                format!(
                    "{}{}: {kind}{}",
                    argument.name,
                    if argument.required { "" } else { "?" },
                    if !argument.named {
                        " (positional)"
                    } else if !argument.positional {
                        " (named)"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let output = match self.output {
            Output::Type(pattern) => pattern.to_string(),
            Output::Input => "Input".to_owned(),
            Output::WithElement(parameter) => format!("Input<{parameter}>"),
            Output::Prefix(parameter) => format!("List<{parameter}> (Ranking input stays Ranking)"),
        };
        format!("{} | {name}{generics}({arguments}) -> {output}", self.input)
    }
}

impl fmt::Display for TypePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.write_str("Any"),
            Self::Type(constructor) => write!(f, "{constructor}"),
            Self::Var(name) => f.write_str(name),
            Self::Apply(constructor, args) => write!(
                f,
                "{constructor}<{}>",
                args.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Collection(element) => write!(f, "Collection<{element}>"),
            Self::OneOf(patterns) => write!(
                f,
                "({})",
                patterns
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        }
    }
}
