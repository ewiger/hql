//! Structural step contracts shared by elaboration and the builtin catalogue.

use super::CheckCx;
use crate::ast::Arg;
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
    /// Anything that can be traversed: a collection or a stream.
    Iterable(&'static TypePattern),
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
    /// Keep a sequence's type, since its positions are kept; anything else
    /// gains an order and becomes a list.
    Prefix(&'static str),
    /// Like a prefix, except that a sequence whose type states where its
    /// order came from loses that type, since a new order is not that one.
    Reorder(&'static str),
    /// Apply the input's constructor to the result element type.
    Image(&'static str),
    /// A number of the element's type; the sum of nothing is the `Int` zero.
    Number(&'static str),
    /// One type over a finite input and another over a stream that may not
    /// end: a sequence keeps every guarantee it can prove.
    Finite {
        finite: TypePattern,
        open: TypePattern,
    },
    /// A fold: the accumulator when the initial value is supplied, and
    /// otherwise an optional element, the accumulator then being the element.
    Fold {
        initial: &'static str,
        accumulator: &'static str,
        element: &'static str,
    },
}

impl ArgumentKind {
    /// Whether the argument is written as a function.
    fn is_function(self) -> bool {
        matches!(self, Self::Lambda { .. } | Self::Ordering(_))
    }
}

/// The machine-readable signature used by both checking and help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSpec {
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
            Self::Var(name) => match bindings.get(name).cloned() {
                // A parameter first bound by something narrower, such as an
                // empty list's `List<Never>`, widens to what it meets later.
                Some(expected) if expected.is(actual) && !actual.is(&expected) => {
                    bindings.insert(name, actual.clone());
                    true
                }
                Some(expected) => expected == TypeRef::NEVER || actual.is(&expected),
                None => {
                    bindings.insert(name, actual.clone());
                    true
                }
            },
            Self::Collection(element) => actual
                .element()
                .is_some_and(|actual| element.accept(&actual, bindings)),
            Self::Iterable(element) => actual
                .iterated()
                .is_some_and(|actual| element.accept(&actual, bindings)),
            // An open tree is viewed by its map alternative where a map is
            // asked for; whether this tree is one is confirmed at run time,
            // and a tree that is not fails as a type error there.
            Self::Apply(constructor, patterns)
                if constructor == crate::types::collections::MAP && *actual == TypeRef::DATA =>
            {
                Self::Apply(constructor, patterns)
                    .accept(&constructor.apply([TypeRef::STR, TypeRef::DATA]), bindings)
            }
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

impl OperationSpec {
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
            Output::Prefix(parameter)
            | Output::Reorder(parameter)
            | Output::Image(parameter)
            | Output::Number(parameter) => patterns.push(TypePattern::Var(parameter)),
            Output::Fold {
                accumulator,
                element,
                ..
            } => {
                patterns.push(TypePattern::Var(accumulator));
                patterns.push(TypePattern::Var(element));
            }
            Output::Input => {}
            Output::Finite { finite, open } => {
                patterns.push(finite);
                patterns.push(open);
            }
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
                TypePattern::Collection(element) | TypePattern::Iterable(element) => {
                    patterns.push(*element)
                }
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
            // A whole-input operation over a stream fails for a reason a
            // reader can act on: the input may not end.
            if input.is_stream() && !input.is_collection() {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!(
                        "`{name}` needs a finite input, and {input} may not end; \
                         bound it first with `take(n)`, or batch it with `collect(n)`"
                    ),
                ));
            }
            // Sequences asked for and sets given: a set's members have no
            // positions until something orders them.
            let unordered = input
                .iterated()
                .is_some_and(|element| element.is_collection() && !element.is_sequence());
            let hint = if unordered && self.input.to_string().contains("Seq<") {
                "; a set has no positions, so sort each one first"
            } else {
                ""
            };
            return Err(Diagnostic::typing(
                span.clone(),
                format!("`{name}` needs {}, not {input}{hint}", self.input),
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
                // A lambda is not a value, so written positionally it fills
                // the next function slot and leaves the value slots before it
                // unsupplied: `reduce((a, b) => …)` omits the initial value.
                // A constructor's name is a function and a type at once; where
                // the next slot takes a type, it is the type.
                None if cx.function_arity(argument).is_some()
                    && !positions.clone().next().is_some_and(|index| {
                        matches!(self.arguments[index].kind, ArgumentKind::TypeName(_))
                    }) =>
                {
                    positions.find(|index| self.arguments[*index].kind.is_function())
                }
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
        let folded_from_first = match self.output {
            Output::Fold {
                initial,
                accumulator,
                element,
            } => {
                let omitted = self
                    .arguments
                    .iter()
                    .zip(&supplied)
                    .any(|(spec, argument)| spec.name == initial && argument.is_none());
                if omitted && let Some(bound) = bindings.get(element).cloned() {
                    bindings.insert(accumulator, bound);
                }
                omitted
            }
            _ => false,
        };
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
                    let comparator = cx.function_arity(argument) == Some(2);
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
            // An alias parameter whose type no annotation gave takes the
            // type this argument requires.
            if actual.constructor.0.starts_with("%?") {
                let expected = pattern.instantiate(&bindings, argument_span).ok();
                if cx.placeholder(&actual, expected) {
                    continue;
                }
            }
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
            // A sequence already has the order a prefix is taken in, so it
            // keeps its type; anything else is materialized.
            Output::Prefix(parameter) => {
                if input.is_sequence() {
                    input.clone()
                } else {
                    TypeRef::list(TypePattern::Var(parameter).instantiate(&bindings, span)?)
                }
            }
            Output::Reorder(parameter) => {
                let element = TypePattern::Var(parameter).instantiate(&bindings, span)?;
                if input.is_sequence() && input.args.as_slice() == [element.clone()] {
                    input.clone()
                } else if input.is_sequence() {
                    TypeRef::seq(element)
                } else {
                    TypeRef::list(element)
                }
            }
            Output::Image(parameter) => {
                let element = TypePattern::Var(parameter).instantiate(&bindings, span)?;
                input.image(element)
            }
            Output::Number(parameter) => {
                let element = TypePattern::Var(parameter).instantiate(&bindings, span)?;
                if element == TypeRef::NEVER {
                    TypeRef::INT
                } else {
                    element
                }
            }
            // Only a stream that may not end takes the open form; a finite
            // collection and a single value take the finite one.
            Output::Finite { finite, open } => {
                if input.is_stream() && !input.is_collection() {
                    open.instantiate(&bindings, span)?
                } else {
                    finite.instantiate(&bindings, span)?
                }
            }
            Output::Fold { accumulator, .. } => {
                let result = TypePattern::Var(accumulator).instantiate(&bindings, span)?;
                if folded_from_first {
                    TypeRef::optional(result)
                } else {
                    result
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
        let render = |argument: &ArgumentSpec, substitute: &dyn Fn(String) -> String| {
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
                "{}{}: {}{}",
                argument.name,
                if argument.required { "" } else { "?" },
                substitute(kind),
                if !argument.named {
                    " (positional)"
                } else if !argument.positional {
                    " (named)"
                } else {
                    ""
                }
            )
        };
        let arguments = self
            .arguments
            .iter()
            .map(|argument| render(argument, &|kind| kind))
            .collect::<Vec<_>>()
            .join(", ");
        let call_with = |input: &str, generics: &str, arguments: &str, output: &str| {
            format!("{input} | {name}{generics}({arguments}) -> {output}")
        };
        let call = |input: &str, generics: &str, output: &str| {
            call_with(input, generics, &arguments, output)
        };
        let with = |extra: &[&str]| {
            let all = extra
                .iter()
                .map(|parameter| (*parameter).to_owned())
                .chain(parameters.iter().cloned())
                .collect::<Vec<_>>();
            format!("<{}>", all.join(", "))
        };
        let input = self.input.to_string();
        // One line per overload, most specific first, in the form issue 0018
        // settles: `S` is the input's own type, `F<_>` its constructor.
        let lines = match self.output {
            Output::Type(pattern) => vec![call(&input, &generics, &pattern.to_string())],
            Output::Number(parameter) => vec![call(&input, &generics, parameter)],
            Output::Input => match self.input {
                TypePattern::Var(parameter) => vec![call(parameter, &generics, parameter)],
                _ => vec![call(&format!("S : {input}"), &with(&["S"]), "S")],
            },
            Output::Image(parameter) => vec![call(
                &format!("F<{}> : {input}", self.element_name()),
                &with(&["F<_>"]),
                &format!("F<{parameter}>"),
            )],
            Output::Finite { finite, open } => {
                let element = self.element_name();
                vec![
                    call(&format!("Stream<{element}>"), &generics, &open.to_string()),
                    call(&format!("Seq<{element}>"), &generics, &finite.to_string()),
                ]
            }
            Output::Prefix(parameter) => vec![
                call(&format!("S : Seq<{parameter}>"), &with(&["S"]), "S"),
                call(&input, &generics, &format!("List<{parameter}>")),
            ],
            Output::Reorder(parameter) => vec![
                call(
                    &format!("Ranking<U> : Seq<{parameter}>"),
                    &with(&["U"]),
                    &format!("Seq<{parameter}>"),
                ),
                call(&format!("S : Seq<{parameter}>"), &with(&["S"]), "S"),
                call(&input, &generics, &format!("List<{parameter}>")),
            ],
            // Without the initial value the accumulator is the element, so the
            // second overload is written with one substituted for the other.
            Output::Fold {
                initial,
                accumulator,
                element,
            } => {
                let from_first = self
                    .arguments
                    .iter()
                    .filter(|argument| argument.name != initial)
                    .map(|argument| {
                        render(argument, &|kind: String| kind.replace(accumulator, element))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let generics_from_first = format!(
                    "<{}>",
                    parameters
                        .iter()
                        .filter(|parameter| parameter.as_str() != accumulator)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let with_initial = self
                    .arguments
                    .iter()
                    .map(|argument| {
                        let required = ArgumentSpec {
                            required: true,
                            ..*argument
                        };
                        render(&required, &|kind| kind)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![
                    call_with(&input, &generics, &with_initial, accumulator),
                    call_with(
                        &input,
                        &generics_from_first,
                        &from_first,
                        &format!("Option<{element}>"),
                    ),
                ]
            }
        };
        lines.join("\n")
    }

    /// The element parameter of a collection input, for printing.
    fn element_name(&self) -> String {
        match self.input {
            TypePattern::Collection(element) | TypePattern::Iterable(element) => {
                element.to_string()
            }
            _ => "T".to_owned(),
        }
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
            Self::Iterable(element) => write!(f, "Iterable<{element}>"),
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
