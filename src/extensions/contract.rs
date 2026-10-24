//! An operation's checking signature, derived from its HQL declaration.
//!
//! A standard module's `.hql` file is the only place a function's types are
//! written. Binding reads each external declaration once and turns it into
//! the structural signature the checker consumes: generic parameters and
//! their bounds, the input, the arguments in order, a function type as a
//! lambda and `...T` as a variadic argument, and the result. The input is the
//! receiver of a declaration that takes `self`, typed by the module's pattern
//! or written; for a declaration without `self` it is the first argument,
//! which is how a call and a pipeline step lower, and binding records that
//! the declaration has no receiver, so `value.f()` does not reach it.

use super::spec::{
    ArgumentKind as A, ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern as P,
};
use crate::ast::{ExternalDecl, Param, Passing, TypeAnn};
use crate::types::TypeRef;
use crate::types::program::intern;

/// The signature an external declaration states, or why it states none the
/// checker can consume.
pub(crate) fn derive(
    declaration: &ExternalDecl,
    receiver: Option<&TypeAnn>,
) -> Result<OperationSpec, String> {
    let mut variables: Vec<String> = declaration.generics.clone();
    // A receiver module's own parameters, `T` of `module option for Option<T>`, are
    // variables of every function in it.
    if let Some(receiver) = receiver {
        variables.extend(
            receiver
                .arguments
                .iter()
                .filter(|argument| {
                    argument.arguments.is_empty()
                        && crate::types::builtin::constructor(&argument.name).is_none()
                })
                .map(|argument| argument.name.clone()),
        );
    }
    let mut seen = std::collections::HashSet::new();
    variables.retain(|name| seen.insert(name.clone()));
    // A bound written with arguments is a shape: `S : Seq<T>` holds what
    // meets `Seq<T>` at its own type, and `F<_> : Iterable<_>` is the
    // constructor of what an Iterable holds.
    let mut scope = Scope {
        variables: variables.clone(),
        shapes: Vec::new(),
        families: Vec::new(),
    };
    for (name, bound) in &declaration.bounds {
        if bound.is_constructor() {
            let base = crate::types::builtin::constructor(&bound.name)
                .ok_or_else(|| format!("unknown bound `{}`", bound.name))?;
            scope
                .families
                .push((name.clone(), base, bound.arguments.len()));
        } else if !bound.arguments.is_empty() {
            scope.shapes.push((name.clone(), bound.clone()));
        }
    }
    let pattern = |written: &TypeAnn| pattern(written, &scope);
    let mut parameters = Vec::new();
    for name in &variables {
        let bound = declaration
            .bounds
            .iter()
            .find(|(variable, bound)| variable == name && bound.arguments.is_empty())
            .map(|(_, bound)| {
                crate::types::builtin::constructor(&bound.name)
                    .ok_or_else(|| format!("unknown bound `{}`", bound.name))
            })
            .transpose()?;
        parameters.push(TypeParameter {
            name: intern(name),
            bound,
        });
    }
    let (input, written): (P, &[Param]) = match (declaration.has_self, receiver) {
        (true, _) if declaration.receiver.is_some() => (
            pattern(declaration.receiver.as_ref().expect("written"))?,
            &declaration.parameters,
        ),
        (true, Some(receiver)) => (pattern(receiver)?, &declaration.parameters),
        (true, None) => return Err("takes `self` outside a receiver module".to_owned()),
        (false, _) => match declaration.parameters.split_first() {
            Some((first, rest)) => (pattern(&first.annotation)?, rest),
            None => return Err("takes no input".to_owned()),
        },
    };
    let mut arguments = Vec::new();
    for parameter in written {
        let kind = match parameter.annotation.name.strip_prefix("Fn") {
            // `T -> Is<U>`: a predicate testing its parameter's type.
            Some("1")
                if !parameter.variadic
                    && matches!(
                        parameter.annotation.arguments.as_slice(),
                        [_, tested] if tested.name == "Is"
                    ) =>
            {
                let [tested, is] = parameter.annotation.arguments.as_slice() else {
                    unreachable!("matched two arguments");
                };
                match is.arguments.as_slice() {
                    [named]
                        if named.arguments.is_empty() && scope.variables.contains(&named.name) =>
                    {
                        A::Narrowing {
                            parameter: pattern(tested)?,
                            narrowed: intern(&named.name),
                        }
                    }
                    _ => return Err("`Is<U>` names one type variable".to_owned()),
                }
            }
            Some(arity) if arity.parse::<usize>().is_ok() && !parameter.variadic => {
                let (result, inputs) = parameter
                    .annotation
                    .arguments
                    .split_last()
                    .ok_or_else(|| "a function type without a result".to_owned())?;
                A::Lambda {
                    parameters: inputs
                        .iter()
                        .map(&pattern)
                        .collect::<Result<Vec<_>, _>>()?
                        .leak(),
                    result: pattern(result)?,
                }
            }
            // `Type<U>` takes a type name and binds U to the type it names.
            _ if parameter.annotation.name == "Type" => {
                match parameter.annotation.arguments.as_slice() {
                    [named]
                        if named.arguments.is_empty() && scope.variables.contains(&named.name) =>
                    {
                        A::TypeName(intern(&named.name))
                    }
                    _ => return Err("`Type<U>` names one type variable".to_owned()),
                }
            }
            _ => A::Value(pattern(&parameter.annotation)?),
        };
        arguments.push(ArgumentSpec {
            name: intern(&parameter.name),
            kind,
            required: !parameter.variadic && parameter.default.is_none(),
            positional: parameter.passing != Passing::Name,
            named: !parameter.variadic && parameter.passing != Passing::Position,
            variadic: parameter.variadic,
        });
    }
    Ok(OperationSpec {
        parameters: parameters.leak(),
        input,
        arguments: arguments.leak(),
        output: O::Type(pattern(&declaration.result)?),
    })
}

/// The type variables of a declaration, and those its bounds give a shape.
struct Scope {
    variables: Vec<String>,
    shapes: Vec<(String, TypeAnn)>,
    families: Vec<(String, crate::types::TypeConstructor, usize)>,
}

fn pattern(written: &TypeAnn, scope: &Scope) -> Result<P, String> {
    let variables = &scope.variables;
    if let Some((name, base, arity)) = scope
        .families
        .iter()
        .find(|(name, _, _)| *name == written.name)
    {
        if written.arguments.len() != *arity {
            return Err(format!(
                "`{name}` is a constructor of {arity} argument{}: `{name}<{}>`",
                if *arity == 1 { "" } else { "s" },
                vec!["_"; *arity].join(", ")
            ));
        }
        let elements = written
            .arguments
            .iter()
            .map(|element| pattern(element, scope))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(P::Family(intern(name), *base, elements.leak()));
    }
    if written.arguments.is_empty()
        && let Some((name, shape)) = scope.shapes.iter().find(|(name, _)| *name == written.name)
    {
        return Ok(P::Is(
            intern(name),
            Box::leak(Box::new(pattern(shape, scope)?)),
        ));
    }
    if written.arguments.is_empty() && variables.contains(&written.name) {
        return Ok(P::Var(intern(&written.name)));
    }
    // A value of any type, each one at its own: what `...Any` takes in
    // every position, where `...T` binds one type for them all.
    if written.arguments.is_empty() && written.name == "Any" {
        return Ok(P::Any);
    }
    // The structural carriers: any collection, retrieval carriers included,
    // and anything that can be traversed.
    if let [element] = written.arguments.as_slice() {
        match written.name.as_str() {
            "Collection" => {
                return Ok(P::Collection(Box::leak(Box::new(pattern(element, scope)?))));
            }
            "Iterable" => {
                return Ok(P::Iterable(Box::leak(Box::new(pattern(element, scope)?))));
            }
            _ => {}
        }
    }
    let constructor = crate::types::builtin::constructor(&written.name)
        .ok_or_else(|| format!("unknown type `{}`", written.name))?;
    if written.arguments.is_empty() {
        return Ok(P::Type(constructor));
    }
    let arguments = written
        .arguments
        .iter()
        .map(|argument| pattern(argument, scope))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(P::Apply(constructor, arguments.leak()))
}

/// Where a native signature departs from the one its declaration states, as
/// far as the declaration states one: the input, each argument's name, kind,
/// whether it may be omitted and whether it is variadic, the generic bounds, and the result where the native
/// output is a type. An argument only Rust can describe, a type name or a
/// value of any type, is not compared.
pub(crate) fn departures(native: &OperationSpec, declared: &OperationSpec) -> Vec<String> {
    let mut found = Vec::new();
    if native.input != declared.input {
        found.push(format!(
            "the input is {:?} natively and {:?} as declared",
            native.input, declared.input
        ));
    }
    if native.arguments.len() != declared.arguments.len() {
        found.push(format!(
            "{} arguments natively and {} as declared",
            native.arguments.len(),
            declared.arguments.len()
        ));
    }
    for (held, written) in native.arguments.iter().zip(declared.arguments) {
        if matches!(held.kind, A::TypeName(_) | A::Value(P::Any)) {
            continue;
        }
        if held.name != written.name
            || held.kind != written.kind
            || held.variadic != written.variadic
            || held.required != written.required
            || held.positional != written.positional
            || held.named != written.named
        {
            found.push(format!(
                "argument `{}` is {:?} natively and `{}` {:?} as declared",
                held.name, held.kind, written.name, written.kind
            ));
        }
    }
    for parameter in native.parameters {
        let declared_bound = declared
            .parameters
            .iter()
            .find(|held| held.name == parameter.name)
            .and_then(|held| held.bound);
        if parameter.bound != declared_bound {
            found.push(format!(
                "`{}` is bounded by {:?} natively and {:?} as declared",
                parameter.name, parameter.bound, declared_bound
            ));
        }
    }
    if let O::Type(output) = native.output
        && native.output != declared.output
    {
        found.push(format!(
            "the result is {output:?} natively and {:?} as declared",
            declared.output
        ));
    }
    found
}

/// Whether an earlier declaration of a name takes every call a later one
/// fits, so the later one is never taken: the earlier takes the same
/// arguments, at least as many of them may be omitted, and its input and
/// each value argument accept everything the later one's do. Where that
/// cannot be told from the patterns alone the answer is no.
pub(crate) fn shadows(earlier: &OperationSpec, later: &OperationSpec) -> bool {
    earlier.arguments.len() == later.arguments.len()
        && covers(earlier.input, later.input, earlier, later)
        && earlier
            .arguments
            .iter()
            .zip(later.arguments)
            .all(|(first, second)| {
                first.name == second.name
                    && first.positional == second.positional
                    && first.named == second.named
                    && first.variadic == second.variadic
                    && (!first.required || second.required)
                    && match (first.kind, second.kind) {
                        (A::Value(first), A::Value(second)) => {
                            covers(first, second, earlier, later)
                        }
                        // A function is chosen by how many parameters it
                        // takes, and a type name by being one.
                        (
                            A::Lambda {
                                parameters: first, ..
                            },
                            A::Lambda {
                                parameters: second, ..
                            },
                        ) => first.len() == second.len(),
                        (A::TypeName(_), A::TypeName(_))
                        | (A::Narrowing { .. }, A::Narrowing { .. }) => true,
                        _ => false,
                    }
            })
}

/// Whether a pattern of the earlier declaration accepts every type a
/// pattern of the later one does.
fn covers(first: P, second: P, earlier: &OperationSpec, later: &OperationSpec) -> bool {
    if first == second && same_bounds(first, earlier, later) {
        return true;
    }
    let below = |narrower: crate::types::TypeConstructor, wider: crate::types::TypeConstructor| {
        crate::types::TypeRef::from(narrower).is(&crate::types::TypeRef::from(wider))
    };
    match (first, second) {
        (P::Any, _) => true,
        (P::Var(name), second) => {
            once(name, earlier)
                && match bound(earlier, name) {
                    None => true,
                    Some(wider) => matches!(second, P::Type(narrower) if below(narrower, wider)),
                }
        }
        (P::Type(wider), P::Type(narrower)) => below(narrower, wider),
        (P::Collection(first), P::Collection(second))
        | (P::Iterable(first), P::Iterable(second) | P::Collection(second)) => {
            covers(*first, *second, earlier, later)
        }
        // A constructor whose one argument is its element: `Seq<T>`, `Set<T>`.
        (P::Collection(first), P::Apply(narrower, [second]))
            if narrower.apply([TypeRef::NEVER]).element() == Some(TypeRef::NEVER) =>
        {
            covers(*first, *second, earlier, later)
        }
        (P::Iterable(first), P::Apply(narrower, [second]))
            if narrower.apply([TypeRef::NEVER]).iterated() == Some(TypeRef::NEVER) =>
        {
            covers(*first, *second, earlier, later)
        }
        (P::Apply(wider, firsts), P::Apply(narrower, seconds)) => {
            wider == narrower
                && firsts.len() == seconds.len()
                && firsts
                    .iter()
                    .zip(seconds)
                    .all(|(first, second)| covers(*first, *second, earlier, later))
        }
        (P::Family(_, wider, firsts), P::Family(_, narrower, seconds)) => {
            below(narrower, wider)
                && firsts.len() == seconds.len()
                && firsts
                    .iter()
                    .zip(seconds)
                    .all(|(first, second)| covers(*first, *second, earlier, later))
        }
        (P::Is(_, first), P::Is(_, second)) => covers(*first, *second, earlier, later),
        (P::Is(_, first), second) => covers(*first, second, earlier, later),
        (first, P::Is(_, second)) => covers(first, *second, earlier, later),
        (P::OneOf(alternatives), second) => alternatives
            .iter()
            .any(|first| covers(*first, second, earlier, later)),
        _ => false,
    }
}

/// Whether every parameter a pattern names has one bound in both
/// declarations.
fn same_bounds(pattern: P, earlier: &OperationSpec, later: &OperationSpec) -> bool {
    earlier
        .parameters
        .iter()
        .filter(|parameter| occurrences(pattern, parameter.name) > 0)
        .all(|parameter| bound(earlier, parameter.name) == bound(later, parameter.name))
}

/// The bound of a declaration's parameter, when it has one.
fn bound(spec: &OperationSpec, name: &str) -> Option<crate::types::TypeConstructor> {
    spec.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .and_then(|parameter| parameter.bound)
}

/// Whether a declaration names a parameter in one place only, so nothing
/// but its bound constrains it.
fn once(name: &str, spec: &OperationSpec) -> bool {
    let mut places = occurrences(spec.input, name);
    for argument in spec.arguments {
        places += match argument.kind {
            A::Value(pattern) => occurrences(pattern, name),
            A::TypeName(parameter) => usize::from(parameter == name),
            A::Lambda { parameters, result } => {
                parameters
                    .iter()
                    .map(|pattern| occurrences(*pattern, name))
                    .sum::<usize>()
                    + occurrences(result, name)
            }
            A::Narrowing {
                parameter,
                narrowed,
            } => occurrences(parameter, name) + usize::from(narrowed == name),
        };
    }
    places == 1
}

/// How many times a pattern names a parameter.
fn occurrences(pattern: P, name: &str) -> usize {
    match pattern {
        P::Any | P::Type(_) => 0,
        P::Var(held) => usize::from(held == name),
        P::Apply(_, patterns) | P::OneOf(patterns) => patterns
            .iter()
            .map(|pattern| occurrences(*pattern, name))
            .sum(),
        P::Collection(element) | P::Iterable(element) => occurrences(*element, name),
        P::Is(held, inner) => usize::from(held == name) + occurrences(*inner, name),
        P::Family(held, _, elements) => {
            usize::from(held == name)
                + elements
                    .iter()
                    .map(|pattern| occurrences(*pattern, name))
                    .sum::<usize>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::spec::TypeParameter;
    use crate::types::TypeConstructor;

    fn declared(source: &str) -> Result<OperationSpec, String> {
        let program = crate::parser::parse(source).expect("parses");
        let receiver = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                crate::ast::Stmt::Module { pattern, .. } => pattern.as_ref(),
                _ => None,
            });
        let declaration = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                crate::ast::Stmt::External(declaration) => Some(declaration),
                _ => None,
            })
            .expect("a declaration");
        derive(declaration, receiver)
    }

    const STRING: P = P::Type(TypeConstructor("String"));
    const INT: P = P::Type(TypeConstructor("Int"));

    #[test]
    fn a_receiver_function_takes_the_module_pattern_as_its_input() {
        let spec = declared(
            "module string for String\nfn pad_left(self, width: Int, fill: String) : String",
        )
        .unwrap();
        assert_eq!(spec.input, STRING);
        assert_eq!(spec.output, O::Type(STRING));
        let arguments: Vec<_> = spec
            .arguments
            .iter()
            .map(|held| (held.name, held.kind))
            .collect();
        assert_eq!(
            arguments,
            [("width", A::Value(INT)), ("fill", A::Value(STRING))]
        );
    }

    #[test]
    fn a_written_receiver_a_lambda_and_a_bound_are_read_from_the_declaration() {
        let spec = declared(
            "module seq for Seq<T>\nfn min_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>",
        )
        .unwrap();
        assert_eq!(spec.input, P::Collection(&P::Var("T")));
        assert_eq!(
            spec.arguments[0].kind,
            A::Lambda {
                parameters: &[P::Var("T")],
                result: P::Var("K"),
            }
        );
        assert!(spec.parameters.contains(&TypeParameter {
            name: "K",
            bound: Some(TypeConstructor("Orderable")),
        }));
        let variadic =
            declared("module string for String\nfn fmt(self, args: ...Any) : String").unwrap();
        assert!(variadic.arguments[0].variadic && !variadic.arguments[0].required);
        assert_eq!(variadic.arguments[0].kind, A::Value(P::Any));
        let one = declared("fn f<T>(x: Int, args: ...T) : Int").unwrap();
        assert_eq!(one.arguments[0].kind, A::Value(P::Var("T")));
    }

    #[test]
    fn a_shaped_bound_a_constructor_parameter_and_a_type_argument_are_read() {
        let kept = declared("fn take<S : Seq<T>, T>(xs: S, n: Int) : S").unwrap();
        let shape = P::Is(
            "S",
            &P::Apply(crate::types::collections::SEQ, &[P::Var("T")]),
        );
        assert_eq!(kept.input, shape);
        assert_eq!(kept.output, O::Type(shape));
        let mapped =
            declared("fn map<F<_> : Iterable<_>, T, R>(xs: F<T>, by: T -> R) : F<R>").unwrap();
        let iterable = crate::types::collections::ITERABLE;
        assert_eq!(mapped.input, P::Family("F", iterable, &[P::Var("T")]));
        assert_eq!(
            mapped.output,
            O::Type(P::Family("F", iterable, &[P::Var("R")]))
        );
        let narrowed = declared("fn is<T, U>(x: T, type: Type<U>) : Bool").unwrap();
        assert_eq!(narrowed.arguments[0].kind, A::TypeName("U"));
        let unbound = crate::parser::parse("fn f<F<_>>(xs: F<Int>) : Int").unwrap_err();
        assert!(
            unbound.message().contains("needs its bound"),
            "{}",
            unbound.message()
        );
    }

    #[test]
    fn a_native_signature_that_differs_from_its_declaration_is_named() {
        let declared = declared("fn sign<N>(x: N) : Int").unwrap();
        let native = OperationSpec {
            parameters: &[],
            input: P::OneOf(&[INT, P::Type(TypeConstructor("Float"))]),
            arguments: &[],
            output: O::Type(INT),
        };
        let found = departures(&native, &declared);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("the input is"), "{found:?}");
        assert!(departures(&declared, &declared).is_empty());
    }

    #[test]
    fn a_parameter_with_a_default_may_be_omitted_and_only_a_declaration_has_one() {
        let spec = declared("fn range(start: Int, end: Int, step: Int = 1) : List<Int>").unwrap();
        let required: Vec<_> = spec
            .arguments
            .iter()
            .map(|held| (held.name, held.required))
            .collect();
        assert_eq!(required, [("end", true), ("step", false)]);
        let body = crate::parser::parse("fn f(n: Int = 1) : Int = n").unwrap_err();
        assert!(
            body.message()
                .contains("a default belongs to an external declaration"),
            "{}",
            body.message()
        );
    }

    #[test]
    fn a_slash_ends_the_positional_parameters_and_a_star_opens_the_named_ones() {
        let spec = declared("fn f(x: Int, a: Int, /, b: Int, *, c: Int = 0) : Int").unwrap();
        let passing: Vec<_> = spec
            .arguments
            .iter()
            .map(|held| (held.name, held.positional, held.named))
            .collect();
        assert_eq!(
            passing,
            [("a", true, false), ("b", true, true), ("c", false, true)]
        );
        for (source, message) in [
            (
                "fn f(x: Int, a: Int, /) : Int = a",
                "belong to an external declaration",
            ),
            (
                "fn f(x: Int, *, a: Int) : Int = a",
                "belong to an external declaration",
            ),
            ("fn f(/, x: Int) : Int", "`/` follows the parameters"),
            (
                "fn f(x: Int, *, a: Int, /) : Int",
                "`/` follows the parameters",
            ),
        ] {
            let refused = crate::parser::parse(source).unwrap_err();
            assert!(refused.message().contains(message), "{}", refused.message());
        }
    }

    #[test]
    fn an_overload_an_earlier_one_takes_every_call_of_is_shadowed() {
        let shadowed = |earlier: &str, later: &str| {
            shadows(&declared(earlier).unwrap(), &declared(later).unwrap())
        };
        for (earlier, later) in [
            ("fn f(x: Int, n: Int) : Int", "fn f(x: Int, n: Int) : Int"),
            ("fn f<T>(x: T) : Int", "fn f(x: Int) : Int"),
            ("fn f<T : Orderable>(x: T) : Int", "fn f(x: Int) : Int"),
            (
                "fn take<T>(xs: Collection<T>, n: Int) : List<T>",
                "fn take<S : Seq<T>, T>(xs: S, n: Int) : S",
            ),
            (
                "fn f<T>(xs: Collection<T>, n: Int = 1) : Int",
                "fn f<T>(xs: List<T>, n: Int) : Int",
            ),
        ] {
            assert!(shadowed(earlier, later), "{earlier} / {later}");
        }
        for (earlier, later) in [
            ("fn abs(x: Int) : Int", "fn abs(x: Float) : Float"),
            ("fn f<T : Orderable>(x: T) : Int", "fn f(x: Bool) : Int"),
            (
                "fn sum(xs: Collection<Int>) : Int",
                "fn sum(xs: Collection<Float>) : Float",
            ),
            (
                "fn take<S : Seq<T>, T>(xs: S, n: Int) : S",
                "fn take<T>(xs: Collection<T>, n: Int) : List<T>",
            ),
            (
                "fn f<T>(xs: Collection<T>, n: Int) : Int",
                "fn f<T>(xs: List<T>, n: Int = 1) : Int",
            ),
            (
                "fn count<T>(xs: Collection<T>, value: T) : Int",
                "fn count(xs: List<Int>, value: Int) : Int",
            ),
            (
                "fn f<T>(xs: List<T>, by: T -> Bool) : Int",
                "fn f<T>(xs: List<T>, by: (T, T) -> Bool) : Int",
            ),
        ] {
            assert!(!shadowed(earlier, later), "{earlier} / {later}");
        }
    }

    #[test]
    fn a_constructor_parameter_may_take_several_arguments() {
        let spec = declared(
            "fn map_values<M<_, _> : Map<_, _>, K, V, U>(m: M<K, V>, by: V -> U) : M<K, U>",
        )
        .unwrap();
        spec.validate().unwrap();
        let map = crate::types::collections::MAP;
        assert_eq!(spec.input, P::Family("M", map, &[P::Var("K"), P::Var("V")]));
        assert!(
            spec.signature("map_values")
                .starts_with("M<K, V> : Map<K, V> | map_values<M<_, _>")
        );
        let mut bindings = std::collections::BTreeMap::new();
        let ordered = crate::types::builtin::named(
            "OrderedMap",
            &[crate::types::TypeRef::STR, crate::types::TypeRef::INT],
        )
        .unwrap();
        assert!(spec.input.accept(&ordered, &mut bindings));
        bindings.insert("U", crate::types::TypeRef::BOOL);
        let O::Type(output) = spec.output else {
            panic!("a type")
        };
        let rebuilt = output.instantiate(&bindings, &(0..0)).unwrap();
        assert_eq!(rebuilt.to_string(), "OrderedMap<String, Bool>");
        let wrong = declared("fn f<M<_, _> : Map<_, _>, K>(m: M<K>) : Int").unwrap_err();
        assert!(wrong.contains("a constructor of 2 arguments"), "{wrong}");
    }

    #[test]
    fn a_declaration_without_an_input_or_with_an_unknown_type_states_no_signature() {
        assert!(declared("fn now() : Int").unwrap_err().contains("no input"));
        assert!(
            declared("fn f(x: Nothing) : Int")
                .unwrap_err()
                .contains("unknown type `Nothing`")
        );
    }
}
