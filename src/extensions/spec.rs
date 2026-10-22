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
    /// A parameter bound by a shape, `S : Seq<T>`: what meets the shape,
    /// held at its own type.
    Is(&'static str, &'static TypePattern),
    /// A constructor parameter applied to an element, `F<T>` with
    /// `F<_> : Iterable<_>`: anything the bound's constructor holds, whose
    /// constructor `F<U>` rebuilds around another element.
    Family(&'static str, TypeConstructor, &'static TypePattern),
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
    /// Whether every further positional argument is one more of it: the
    /// values `fmt` writes into its template.
    pub variadic: bool,
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
            variadic: false,
        }
    }
}

/// The result expression, including operations that retain input structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// Instantiate a declared type pattern.
    Type(TypePattern),
    /// One type over a finite input and another over a stream that may not
    /// end: a sequence keeps every guarantee it can prove.
    Finite {
        finite: TypePattern,
        open: TypePattern,
    },
    /// A type the providing extension computes from the call, named by the
    /// result written in help: what `source` reads depends on its endpoint
    /// and its format.
    Computed(&'static str),
}

impl ArgumentKind {
    /// Whether the argument is written as a function.
    fn is_function(self) -> bool {
        matches!(self, Self::Lambda { .. })
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
    /// The input as written, when it was: `file("x.json")` before `| source`.
    pub input_expr: Option<&'a crate::ast::Expr>,
    pub arguments: &'a [Arg],
    pub bindings: BTreeMap<&'static str, TypeRef>,
}

/// A type viewed as one of its supertypes, with that constructor's own
/// arguments: `Ranking<Card>` as a `Seq` is `Seq<Hit<Card>>`.
fn viewed(actual: &TypeRef, constructor: TypeConstructor) -> Option<TypeRef> {
    let system = crate::types::builtin::system().ok()?;
    system.as_supertype(actual, constructor).ok().flatten()
}

/// The input viewed as a constructor shape, `Seq<T>`, where it has that
/// constructor in its ancestry.
fn viewed_shape(shape: &TypePattern, actual: &TypeRef) -> Option<TypeRef> {
    match shape {
        TypePattern::Apply(constructor, _) => viewed(actual, *constructor),
        _ => None,
    }
}

/// The refusal of an input no signature of an operation takes, naming what
/// each one takes.
pub(crate) fn refused(
    name: &str,
    inputs: &[TypePattern],
    input: &TypeRef,
    span: &Range<usize>,
) -> Diagnostic {
    // A whole-input operation over a stream fails for a reason a reader can
    // act on: the input may not end.
    if input.is_stream() && !input.is_collection() {
        return Diagnostic::typing(
            span.clone(),
            format!(
                "`{name}` needs a finite input, and {input} may not end; \
                 bound it first with `take(n)`, or batch it with `collect(n)`"
            ),
        );
    }
    let mut wanted: Vec<String> = Vec::new();
    for pattern in inputs {
        let written = pattern.to_string();
        if !wanted.contains(&written) {
            wanted.push(written);
        }
    }
    // Sequences asked for and sets given: a set's members have no positions
    // until something orders them.
    let unordered = input
        .iterated()
        .is_some_and(|element| element.is_collection() && !element.is_sequence());
    let hint = if unordered && wanted.iter().any(|written| written.contains("Seq<")) {
        "; a set has no positions, so sort each one first"
    } else {
        ""
    };
    Diagnostic::typing(
        span.clone(),
        format!("`{name}` needs {}, not {input}{hint}", wanted.join(" or ")),
    )
}

impl TypePattern {
    /// Whether the pattern refers to a parameter.
    fn mentions(self, parameter: &str) -> bool {
        match self {
            Self::Any | Self::Type(_) => false,
            Self::Var(name) => name == parameter,
            Self::Apply(_, patterns) | Self::OneOf(patterns) => {
                patterns.iter().any(|pattern| pattern.mentions(parameter))
            }
            Self::Collection(element) | Self::Iterable(element) => element.mentions(parameter),
            Self::Is(name, inner) | Self::Family(name, _, inner) => {
                name == parameter || inner.mentions(parameter)
            }
        }
    }

    /// Whether a type fits the pattern, with nothing bound.
    pub(crate) fn accepts(self, actual: &TypeRef) -> bool {
        self.accept(actual, &mut BTreeMap::new())
    }

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
            Self::Is(name, shape) => {
                if bindings.contains_key(name) {
                    return Self::Var(name).accept(actual, bindings);
                }
                // A shape's arguments are read off the input viewed as the
                // shape: a `Ranking<T>` meets `Seq<Hit<T>>`.
                // An open tree, which has no ancestry to view, meets a map
                // shape as the shape itself says.
                let met = match (*shape, viewed_shape(shape, actual)) {
                    (Self::Apply(_, patterns), Some(view)) => {
                        patterns.len() == view.args.len()
                            && patterns
                                .iter()
                                .zip(&view.args)
                                .all(|(pattern, actual)| pattern.accept(actual, bindings))
                    }
                    _ => shape.accept(actual, bindings),
                };
                if !met {
                    return false;
                }
                bindings.insert(name, actual.clone());
                true
            }
            Self::Family(name, base, element) => {
                let Some(view) = viewed(actual, base) else {
                    return false;
                };
                let [held] = view.args.as_slice() else {
                    return false;
                };
                if !element.accept(held, bindings) {
                    return false;
                }
                bindings.entry(name).or_insert_with(|| actual.clone());
                true
            }
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
            Self::Is(name, _) => Self::Var(name).instantiate(bindings, span),
            // The input's own constructor around the new element: a set maps
            // to a set, and a ranking, whose argument is not its element, to
            // a sequence.
            Self::Family(name, _, element) => {
                let held = Self::Var(name).instantiate(bindings, span)?;
                Ok(held.image(element.instantiate(bindings, span)?))
            }
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
        if let Some(position) = self.arguments.iter().position(|argument| argument.variadic)
            && (position + 1 != self.arguments.len()
                || !self.arguments[position].positional
                || !matches!(self.arguments[position].kind, ArgumentKind::Value(_)))
        {
            return Err(SpecError(format!(
                "variadic argument `{}` must be the last, positional, and a value",
                self.arguments[position].name
            )));
        }
        for argument in self.arguments {
            if !names.insert(argument.name) || (!argument.positional && !argument.named) {
                return Err(SpecError(format!(
                    "duplicate or unreachable argument `{}`",
                    argument.name
                )));
            }
            match argument.kind {
                ArgumentKind::Value(pattern) => patterns.push(pattern),
                ArgumentKind::TypeName(parameter) => patterns.push(TypePattern::Var(parameter)),
                ArgumentKind::Lambda { parameters, result } => {
                    patterns.extend(parameters);
                    patterns.push(result);
                }
            }
        }
        match self.output {
            Output::Type(pattern) => patterns.push(pattern),
            Output::Computed(_) => {}
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
                TypePattern::Is(name, inner) | TypePattern::Family(name, _, inner) => {
                    if !parameters.contains(name) {
                        return Err(SpecError(format!("undeclared parameter `{name}`")));
                    }
                    patterns.push(*inner);
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
        input_expr: Option<&'a crate::ast::Expr>,
        arguments: &'a [Arg],
        span: &Range<usize>,
    ) -> Result<(TypeRef, CheckedCall<'a>), Diagnostic> {
        let mut bindings = BTreeMap::new();
        if !self.input.accept(input, &mut bindings) {
            return Err(refused(name, &[self.input], input, span));
        }
        let (supplied, trailing) = self.assign(name, cx, arguments)?;
        let variadic = self.arguments.iter().position(|argument| argument.variadic);
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
        if let Some(variadic) = variadic {
            let ArgumentKind::Value(pattern) = self.arguments[variadic].kind else {
                unreachable!("validated as a value");
            };
            for argument in trailing {
                let actual = cx.infer(&argument.value)?;
                if !pattern.accept(&actual, &mut bindings) {
                    return Err(Diagnostic::typing(
                        argument.value.span.clone(),
                        format!(
                            "`{name}` argument `{}` needs {pattern}, not {actual}",
                            self.arguments[variadic].name
                        ),
                    ));
                }
            }
        }
        for parameter in self.parameters {
            // A parameter only an omitted argument would bind stays unbound.
            if let Some(bound) = parameter.bound
                && bindings.contains_key(parameter.name)
            {
                let actual = TypePattern::Var(parameter.name).instantiate(&bindings, span)?;
                if !actual.is(&TypeRef::from(bound)) {
                    // The element of the input lacking an order is the
                    // common case, and supplying one is the remedy.
                    let message = if TypeRef::from(bound) == TypeRef::ORDERABLE
                        && self.input.mentions(parameter.name)
                    {
                        format!(
                            "`{name}` needs elements with an order of their own, and {actual} \
                             has none; sort with an explicit key first"
                        )
                    } else {
                        format!(
                            "`{name}` requires {} : {bound}, not {actual}",
                            parameter.name
                        )
                    };
                    return Err(Diagnostic::typing(span.clone(), message));
                }
            }
        }
        let result = match self.output {
            Output::Type(pattern) => pattern.instantiate(&bindings, span)?,
            // Only a stream that may not end takes the open form; a finite
            // collection and a single value take the finite one.
            Output::Finite { finite, open } => {
                if input.is_stream() && !input.is_collection() {
                    open.instantiate(&bindings, span)?
                } else {
                    finite.instantiate(&bindings, span)?
                }
            }
            Output::Computed(_) => TypeRef::UNIT,
        };
        let call = CheckedCall {
            input,
            input_expr,
            arguments,
            bindings,
        };
        let result = match self.output {
            Output::Computed(_) => super::io::result(name, &call)
                .map_err(|message| Diagnostic::typing(span.clone(), message))?,
            _ => result,
        };
        Ok((result, call))
    }

    /// Whether a call can take this signature: its input meets the declared
    /// input, every argument has a place, every required one is supplied, and
    /// a function written in place takes as many parameters as its place
    /// does. Nothing is checked beyond that, so choosing a signature infers
    /// nothing.
    pub(crate) fn fits(&self, cx: &mut dyn CheckCx, input: &TypeRef, arguments: &[Arg]) -> bool {
        if !self.input.accepts(input) {
            return false;
        }
        let Ok((supplied, _)) = self.assign("", cx, arguments) else {
            return false;
        };
        self.arguments
            .iter()
            .zip(supplied)
            .all(|(spec, argument)| match (argument, spec.kind) {
                (None, _) => !spec.required,
                (Some(argument), ArgumentKind::Lambda { parameters, .. }) => cx
                    .function_arity(argument)
                    .is_none_or(|arity| arity == parameters.len()),
                (Some(_), _) => true,
            })
    }

    /// Each argument's place: the supplied argument for each declared one,
    /// and the positional arguments past the last place, which a variadic
    /// argument collects.
    #[allow(clippy::type_complexity)]
    fn assign<'a>(
        &self,
        name: &str,
        cx: &mut dyn CheckCx,
        arguments: &'a [Arg],
    ) -> Result<(Vec<Option<&'a Arg>>, Vec<&'a Arg>), Diagnostic> {
        let mut supplied = vec![None; self.arguments.len()];
        let mut positions = self
            .arguments
            .iter()
            .enumerate()
            .filter(|(_, arg)| arg.positional)
            .map(|(index, _)| index);
        let variadic = self.arguments.iter().position(|argument| argument.variadic);
        let mut trailing: Vec<&Arg> = Vec::new();
        for argument in arguments {
            // Past the last declared position, every further positional
            // argument is one more of the variadic one.
            if let Some(variadic) = variadic
                && argument.name.is_none()
                && supplied[variadic].is_some()
            {
                trailing.push(argument);
                continue;
            }
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
        Ok((supplied, trailing))
    }

    /// Render the same contract the checker consumes.
    #[must_use]
    pub fn signature(&self, name: &str) -> String {
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| match (parameter.bound, self.input) {
                (Some(bound), _) => format!("{} : {bound}", parameter.name),
                (None, TypePattern::Family(family, ..)) if family == parameter.name => {
                    format!("{family}<_>")
                }
                (None, _) => parameter.name.to_owned(),
            })
            .collect::<Vec<_>>();
        let generics = if parameters.is_empty() {
            String::new()
        } else {
            format!("<{}>", parameters.join(", "))
        };
        let render = |argument: &ArgumentSpec| {
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
            };
            format!(
                "{}{}: {}{}{}",
                if argument.variadic { "..." } else { "" },
                argument.name,
                if argument.required { "" } else { "?" },
                kind,
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
            .map(render)
            .collect::<Vec<_>>()
            .join(", ");
        let call = |input: &str, generics: &str, output: &str| {
            format!("{input} | {name}{generics}({arguments}) -> {output}")
        };
        // A parameter the input binds is written with its shape where the
        // input is, and by name where it comes back.
        let input = match self.input {
            TypePattern::Is(parameter, shape) => format!("{parameter} : {shape}"),
            TypePattern::Family(parameter, base, element) => {
                format!("{parameter}<{element}> : {base}<{element}>")
            }
            other => other.to_string(),
        };
        let written = |pattern: TypePattern| match pattern {
            TypePattern::Is(parameter, _) => parameter.to_owned(),
            TypePattern::Family(parameter, _, element) => format!("{parameter}<{element}>"),
            other => other.to_string(),
        };
        // One line per overload, most specific first, in the form issue 0018
        // settles: `S` is the input's own type, `F<_>` its constructor.
        let lines = match self.output {
            Output::Type(pattern) => vec![call(&input, &generics, &written(pattern))],
            Output::Computed(written) => vec![call(&input, &generics, written)],
            Output::Finite { finite, open } => {
                let element = self.element_name();
                vec![
                    call(&format!("Stream<{element}>"), &generics, &open.to_string()),
                    call(&format!("Seq<{element}>"), &generics, &finite.to_string()),
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
            // Where a pattern is read as a requirement, a parameter bound by
            // its shape is the shape.
            Self::Is(_, shape) => write!(f, "{shape}"),
            Self::Family(_, base, element) => write!(f, "{base}<{element}>"),
        }
    }
}
