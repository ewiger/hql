//! An operation's checking signature, derived from its HQL declaration.
//!
//! A standard module's `.hql` file is the only place a function's types are
//! written. Binding reads each external declaration once and turns it into
//! the structural signature the checker consumes: generic parameters and
//! their bounds, the input (the receiver of a `module for T`, or the first
//! parameter of a module without one), the arguments in order, a function
//! type as a lambda and `...T` as a variadic argument, and the result.

use super::spec::{
    ArgumentKind as A, ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern as P,
};
use crate::ast::{ExternalDecl, Param, TypeAnn};
use crate::types::program::intern;

/// The signature an external declaration states, or why it states none the
/// checker can consume.
pub(crate) fn derive(
    declaration: &ExternalDecl,
    receiver: Option<&TypeAnn>,
) -> Result<OperationSpec, String> {
    let mut variables: Vec<String> = declaration.generics.clone();
    // A receiver module's own parameters, `T` of `module for Option<T>`, are
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
            scope.families.push((name.clone(), base));
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
            positional: true,
            named: !parameter.variadic,
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
    families: Vec<(String, crate::types::TypeConstructor)>,
}

fn pattern(written: &TypeAnn, scope: &Scope) -> Result<P, String> {
    let variables = &scope.variables;
    if let Some((name, base)) = scope
        .families
        .iter()
        .find(|(name, _)| *name == written.name)
    {
        let [element] = written.arguments.as_slice() else {
            return Err(format!(
                "`{name}` is a constructor of one element: `{name}<T>`"
            ));
        };
        return Ok(P::Family(
            intern(name),
            *base,
            Box::leak(Box::new(pattern(element, scope)?)),
        ));
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
                crate::ast::Stmt::ModuleFor { pattern, .. } => Some(pattern),
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
        let spec =
            declared("module for String\nfn pad_left(self, width: Int, fill: String) : String")
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
            "module for Seq<T>\nfn min_by<K : Orderable>(self: Collection<T>, by: T -> K) : Option<T>",
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
        let variadic = declared("module for String\nfn fmt(self, args: ...Data) : String").unwrap();
        assert!(variadic.arguments[0].variadic && !variadic.arguments[0].required);
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
        assert_eq!(mapped.input, P::Family("F", iterable, &P::Var("T")));
        assert_eq!(
            mapped.output,
            O::Type(P::Family("F", iterable, &P::Var("R")))
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
    fn a_declaration_without_an_input_or_with_an_unknown_type_states_no_signature() {
        assert!(declared("fn now() : Int").unwrap_err().contains("no input"));
        assert!(
            declared("fn f(x: Nothing) : Int")
                .unwrap_err()
                .contains("unknown type `Nothing`")
        );
    }
}
