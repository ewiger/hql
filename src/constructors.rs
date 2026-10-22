//! Checking and evaluating the standard constructors: materialized
//! collections, a graph projected from cards, a channel, and a tree parsed
//! from text.
//!
//! A constructor is declared in `std/` as a function named after its type,
//! beside the type, and binding holds each declaration to the table here:
//! the declarations state the forms a call selects among by its first
//! argument, and this module implements them.

use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::extensions::spec::OperationSpec;
use crate::extensions::{CheckCx, EvalCx};
use crate::types::{MapKind, MapValue, TypeConstructor, TypeRef, Value, builtin, collections};
use std::ops::Range;
use std::sync::Arc;

/// The types a program may name as constructors, abstract ones included so
/// that naming one is refused with its concrete subtypes.
const CONSTRUCTORS: [TypeConstructor; 11] = [
    collections::COLLECTION,
    collections::SEQ,
    collections::LIST,
    collections::SET,
    collections::MAP,
    collections::ORDERED_MAP,
    collections::SORTED_MAP,
    collections::ORDERABLE,
    GRAPH,
    builtin::CHANNEL,
    DATA,
];

const DATA: TypeConstructor = TypeConstructor("Data");

const GRAPH: TypeConstructor = TypeConstructor("Graph");

pub(crate) fn named(name: &str) -> Option<TypeConstructor> {
    CONSTRUCTORS
        .into_iter()
        .find(|constructor| constructor.0 == name)
}

/// Whether a declared name is a constructor's: `UpperCamelCase`, as only a
/// type's name is.
pub(crate) fn is_constructor_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Whether a standard module's declaration of a name is a constructor the
/// binary's table implements: named as a type is, and not an operation of
/// the extension providing the module, as `Logger` of `io.logging` is.
pub(crate) fn is_table_constructor(module: &str, name: &str) -> bool {
    is_constructor_name(name)
        && crate::extensions::builtin_module(module)
            .and_then(|extension| extension.step(name))
            .is_none()
}

/// The forms `std/` declares for a constructor, in the order written: the
/// constructor declarations of the type identity binding bound it to.
pub(crate) fn forms(constructor: TypeConstructor) -> Vec<&'static OperationSpec> {
    crate::extensions::binding::Environment::get()
        .map(|environment| environment.constructor_forms(constructor.0))
        .unwrap_or_default()
}

/// Validate the constructors an environment declares against the table
/// here: every declaration is bound to a concrete type its own module
/// declares, states a form, and returns that type or a subtype of it; each
/// has an implementation here, and each concrete type here is bound to a
/// type identity with a declaration; and two forms of one arity differ in
/// their first parameter.
///
/// # Errors
///
/// The first disagreement, a defect of the binary.
pub(crate) fn bind(environment: &crate::extensions::binding::Environment) -> Result<(), String> {
    let system = builtin::system().map_err(|error| error.to_string())?;
    for constructor in environment.constructors() {
        let module = environment.module(constructor.module).name;
        let name = constructor.name.as_str();
        if constructor.ty.is_none() {
            return Err(format!(
                "`std/{module}` declares the constructor `{name}`, and declares no concrete \
                 type `{name}`: a constructor is declared beside its type"
            ));
        }
        let Some(table) = named(name) else {
            return Err(format!(
                "`std/{module}` declares the constructor `{name}`, and the binary implements \
                 no such constructor"
            ));
        };
        if environment.table(table.0) != constructor.ty {
            return Err(format!(
                "`std/{module}` declares the constructor `{name}`, and the binary's `{name}` \
                 is bound to another module's type `{name}`"
            ));
        }
        if let Err(why) = &constructor.signature {
            return Err(format!(
                "`{module}.{name}` states no signature a constructor can take: {why}"
            ));
        }
        let result = crate::types::builtin::constructor(&constructor.result.name)
            .ok_or_else(|| format!("`{module}.{name}` returns an unknown type"))?;
        let arguments = vec![TypeRef::DATA; constructor.result.arguments.len()];
        let builds = result == table
            || system
                .as_supertype(&result.apply(arguments), table)
                .ok()
                .flatten()
                .is_some();
        if !builds {
            return Err(format!(
                "`std/{module}` declares `{name}` returning {}, which is not a {name}",
                constructor.result.name
            ));
        }
    }
    for constructor in CONSTRUCTORS {
        let concrete = constructor == DATA
            || constructor == GRAPH
            || system
                .definition(constructor)
                .is_ok_and(|definition| definition.kind == crate::types::TypeKind::Concrete);
        let declared = !environment.constructor_forms(constructor.0).is_empty();
        if concrete && !declared {
            return Err(format!(
                "the binary constructs {constructor}, and no module under std/ declares \
                 `fn {constructor}(…)`"
            ));
        }
        if !concrete && declared {
            return Err(format!(
                "std/ declares a constructor of {constructor}, which is abstract"
            ));
        }
        let held = environment.constructor_forms(constructor.0);
        for (index, later) in held.iter().enumerate() {
            if let Some(earlier) = held[..index].iter().find(|earlier| {
                earlier.arguments.len() == later.arguments.len()
                    && earlier.input.to_string() == later.input.to_string()
            }) {
                return Err(format!(
                    "std/ declares `{}` after `{}`: forms of one arity differ in their first \
                     parameter",
                    later.signature(constructor.0),
                    earlier.signature(constructor.0)
                ));
            }
        }
    }
    Ok(())
}

/// The form a call takes: the first whose first parameter, arity and
/// argument types it fits. A call no form takes is refused naming each
/// form's first parameter.
fn select(
    cx: &mut dyn CheckCx,
    constructor: TypeConstructor,
    args: &[Arg],
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    let forms = forms(constructor);
    let Some((first, rest)) = args.split_first() else {
        return Ok(());
    };
    if forms.is_empty() || first.name.is_some() {
        return Ok(());
    }
    let inferred = cx.infer(&first.value)?;
    for input in [&inferred, inferred.present()] {
        for form in &forms {
            if form.fits(cx, input, rest) && form.fits_types(cx, Some(input), rest)? {
                return Ok(());
            }
        }
    }
    Err(Diagnostic::typing(
        span.clone(),
        format!(
            "{constructor} has no form taking {inferred}{}: its forms take {}",
            if rest.is_empty() {
                String::new()
            } else {
                format!(" and {} more", rest.len())
            },
            first_parameters(&forms)
        ),
    ))
}

/// The first parameter of each form, as a refusal names them.
fn first_parameters(forms: &[&OperationSpec]) -> String {
    let mut written: Vec<String> = Vec::new();
    for form in forms {
        let input = form.input.to_string();
        if !written.contains(&input) {
            written.push(input);
        }
    }
    written
        .iter()
        .map(|input| format!("`{input}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A constructor passed as a function value takes the one form of one
/// parameter whose first parameter takes the type the receiving operation
/// passes. None, or several, is refused naming every candidate.
pub(crate) fn resolve_value(
    constructor: TypeConstructor,
    parameter: &TypeRef,
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    let forms: Vec<&OperationSpec> = forms(constructor)
        .into_iter()
        .filter(|form| form.arguments.iter().all(|argument| !argument.required))
        .collect();
    if forms.len() < 2 {
        return Ok(());
    }
    let taking: Vec<&&OperationSpec> = forms
        .iter()
        .filter(|form| form.input.accepts(parameter))
        .collect();
    if taking.len() == 1 {
        return Ok(());
    }
    Err(Diagnostic::typing(
        span.clone(),
        if taking.is_empty() {
            format!(
                "`{constructor}` passed here takes {parameter}, and no form of it does: its \
                 forms take {}",
                first_parameters(&forms)
            )
        } else {
            let candidates: Vec<&OperationSpec> = taking.into_iter().copied().collect();
            format!(
                "`{constructor}` passed here is ambiguous: {parameter} fits its forms taking {}; \
                 write the call, `x => {constructor}(x)`, or give the elements a type",
                first_parameters(&candidates)
            )
        },
    ))
}

/// Check a call of a standard constructor: the binary's own refinements
/// first, a finite input and positions where a `List` needs them, then the
/// form `std/` declares that the call takes.
pub(crate) fn check(
    cx: &mut dyn CheckCx,
    constructor: TypeConstructor,
    args: &[Arg],
    expected: Option<&TypeRef>,
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let built = check_native(cx, constructor, args, expected, span.clone())?;
    select(cx, constructor, args, &span)?;
    Ok(built)
}

fn check_native(
    cx: &mut dyn CheckCx,
    constructor: TypeConstructor,
    args: &[Arg],
    expected: Option<&TypeRef>,
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    if constructor == DATA {
        return check_data(cx, args, span);
    }
    if constructor == GRAPH {
        return check_graph(cx, args, span);
    }
    if constructor == builtin::CHANNEL {
        return check_channel(cx, args, expected, span);
    }
    let system =
        builtin::system().map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    let definition = system
        .definition(constructor)
        .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    if definition.kind != crate::types::TypeKind::Concrete {
        let concrete = CONSTRUCTORS
            .iter()
            .filter(|candidate| {
                system.definition(**candidate).is_ok_and(|found| {
                    found.kind == crate::types::TypeKind::Concrete
                        && found.parameters.len() == definition.parameters.len()
                })
            })
            .filter(|candidate| {
                let arguments = vec![TypeRef::DATA; definition.parameters.len()];
                candidate
                    .apply(arguments.clone())
                    .is(&constructor.apply(arguments))
            })
            .map(|candidate| format!("`{candidate}`"))
            .collect::<Vec<_>>();
        return Err(Diagnostic::typing(
            span,
            if concrete.is_empty() {
                format!("{constructor} is abstract; construct a concrete subtype")
            } else {
                format!(
                    "{constructor} is abstract; construct a concrete subtype: {}",
                    concrete.join(" or ")
                )
            },
        ));
    }
    let arity = if MapKind::of(constructor).is_some() {
        2
    } else {
        1
    };
    if args.len() != arity || args.iter().any(|arg| arg.name.is_some()) {
        return Err(Diagnostic::typing(
            span,
            format!("{constructor} expects {arity} positional collection arguments"),
        ));
    }
    let mut elements = Vec::new();
    for argument in args {
        let input = cx.infer(&argument.value)?;
        if constructor == collections::LIST {
            if input.is_stream() && !input.is_collection() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!(
                        "List needs a finite input, and {input} may not end; bound it with \
                         `take(n)` or batch it with `collect(n)`"
                    ),
                ));
            }
            if input.is_collection() && !input.is_sequence() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!("a List keeps positions, and a {input} has none; `sort` it into one"),
                ));
            }
        }
        if arity == 2 && !input.is_sequence() {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                format!("map keys and values must be sequences, not {input}"),
            ));
        }
        elements.push(input.element().ok_or_else(|| {
            Diagnostic::typing(
                argument.value.span.clone(),
                format!("{constructor} needs a collection, not {input}"),
            )
        })?);
    }
    if let Some(expected) = expected {
        for (actual, wanted) in elements.iter().zip(&expected.args) {
            if !actual.is(wanted) {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!("{actual} does not inhabit constructor argument {wanted}"),
                ));
            }
        }
        system
            .validate_concrete(expected)
            .map_err(|error| Diagnostic::typing(span, error.to_string()))?;
        return Ok(expected.clone());
    }
    system
        .apply(constructor, elements)
        .map_err(|error| Diagnostic::typing(span, error.to_string()))
}

/// `Data(s, F)`: text parsed in a format whose decoder reads `Data`.
fn check_data(
    cx: &mut dyn CheckCx,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let [text, format] = args else {
        return Err(Diagnostic::typing(
            span,
            "`Data` parses text in a format: `Data(s, Json)`, or `s | Data(Json)`",
        ));
    };
    if text.name.is_some() || format.name.as_deref().is_some_and(|name| name != "as") {
        return Err(Diagnostic::typing(
            span,
            "`Data` takes its text by position and its format as `as`",
        ));
    }
    let ty = cx.infer(&text.value)?;
    if !ty.is(&TypeRef::STR) {
        return Err(Diagnostic::typing(
            text.value.span.clone(),
            format!("`Data` parses a String, not {ty}"),
        ));
    }
    let named = cx.type_argument(format)?;
    match crate::io::document_type(named.constructor) {
        Ok(read) if read == TypeRef::DATA => Ok(TypeRef::DATA),
        Ok(read) => Err(Diagnostic::typing(
            format.value.span.clone(),
            format!(
                "{} decodes {read}, not Data: `Data` parses Json, JsonLines, Yaml or Csv",
                named.constructor
            ),
        )),
        Err(_) => Err(Diagnostic::typing(
            format.value.span.clone(),
            format!(
                "{} is no format a Decoder reads: `Data` parses Json, JsonLines, Yaml or Csv",
                named.constructor
            ),
        )),
    }
}

/// `Channel<T>(name)`: a name, and the element type written out, since
/// nothing written to the channel yet can say it.
fn check_channel(
    cx: &mut dyn CheckCx,
    args: &[Arg],
    expected: Option<&TypeRef>,
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let Some(expected) = expected.filter(|expected| expected.args.len() == 1) else {
        return Err(Diagnostic::typing(
            span,
            "a channel is constructed with its element type: `Channel<LogEvent>(\"logs\")`",
        ));
    };
    let [name] = args else {
        return Err(Diagnostic::typing(span, "`Channel<T>` takes one name"));
    };
    let ty = cx.infer(&name.value)?;
    if name.name.is_some() || !ty.is(&TypeRef::STR) {
        return Err(Diagnostic::typing(
            name.value.span.clone(),
            format!("a channel's name is a String, not {ty}"),
        ));
    }
    Ok(expected.clone())
}

/// `Graph(xs)` projects cards, hits or a ranking into a graph, and a graph
/// into itself.
fn check_graph(
    cx: &mut dyn CheckCx,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    if args.iter().any(|argument| argument.name.is_some()) || args.is_empty() || args.len() > 2 {
        return Err(Diagnostic::typing(
            span,
            "Graph takes its nodes and edges, or one collection, by position",
        ));
    }
    let edges_of = |ty: &TypeRef| {
        ty.element()
            .is_some_and(|element| element.is(&TypeRef::EDGE))
    };
    if let [nodes, edges] = args {
        let nodes_type = cx.infer(&nodes.value)?;
        let edges_type = cx.infer(&edges.value)?;
        if !nodes_type
            .element()
            .is_some_and(|element| element.is(&TypeRef::CARD))
        {
            return Err(Diagnostic::typing(
                nodes.value.span.clone(),
                format!("a graph's nodes are cards, not {nodes_type}"),
            ));
        }
        if !edges_of(&edges_type) {
            return Err(Diagnostic::typing(
                edges.value.span.clone(),
                format!("a graph's edges are edges, not {edges_type}"),
            ));
        }
        return Ok(TypeRef::GRAPH);
    }
    let argument = &args[0];
    // A link that may not resolve projects to the graph of what it reaches.
    let inferred = cx.infer(&argument.value)?;
    let input = inferred.present();
    let card = |ty: &TypeRef| {
        ty.is(&TypeRef::DOC)
            || (ty.constructor.0 == "Hit"
                && ty.args.first().is_some_and(|arg| arg.is(&TypeRef::DOC)))
    };
    if input.is(&TypeRef::GRAPH)
        || card(input)
        || input.element().is_some_and(|ty| card(&ty))
        || edges_of(input)
    {
        return Ok(TypeRef::GRAPH);
    }
    Err(Diagnostic::typing(
        argument.value.span.clone(),
        format!("a Graph is projected from cards, hits, edges or a graph, not {input}"),
    ))
}

pub(crate) fn evaluate(
    cx: &mut dyn EvalCx,
    args: &[crate::execution::Arg],
    expected: &TypeRef,
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let constructor = expected.constructor;
    if constructor == DATA {
        return evaluate_data(cx, args, span);
    }
    let mut inputs = Vec::new();
    for argument in args {
        let input = cx.evaluate(&argument.value)?;
        // A constructor materializes, so a traversal's values are produced.
        inputs.push(match input {
            Value::Traversal(traversal) if traversal.forced().is_none() => {
                let mut values = Vec::new();
                cx.traverse(&traversal, &mut |_, value| {
                    values.push(value);
                    Ok(true)
                })?;
                Value::list(values, traversal.element.clone())
            }
            other => other,
        });
    }
    if constructor == builtin::CHANNEL {
        let (Some(Value::Str(name)), Some(element)) = (inputs.first(), expected.args.first())
        else {
            return Err(Diagnostic::runtime(span, "a channel needs a name"));
        };
        crate::io::open_channel(name, element)
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
        return Ok(Value::Endpoint(Arc::new(crate::io::Endpoint::Channel(
            name.to_string(),
            element.clone(),
        ))));
    }
    if constructor == GRAPH {
        if let [nodes, edges] = inputs.as_slice() {
            return crate::extensions::graph::from_edges(Some(nodes), edges, &span);
        }
        let input = inputs
            .pop()
            .ok_or_else(|| Diagnostic::runtime(span.clone(), "Graph needs an argument"))?;
        return crate::extensions::graph::construct(cx.vault(), input, &span);
    }
    let element = |index: usize| expected.args.get(index).cloned().unwrap_or(TypeRef::NEVER);
    let elements = |index: usize| {
        inputs.get(index).and_then(Value::elements).ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                format!("{constructor} needs collection arguments"),
            )
        })
    };
    if let Some(kind) = MapKind::of(constructor) {
        let map = MapValue::new(kind, elements(0)?, elements(1)?, element(0), element(1))
            .map_err(|error| Diagnostic::runtime(span, error.to_string()))?;
        return Ok(Value::Map(Arc::new(map)));
    }
    match constructor {
        collections::LIST => Ok(Value::list(elements(0)?, element(0))),
        collections::SET => Ok(Value::set(elements(0)?, element(0))),
        _ => Err(Diagnostic::runtime(
            span,
            format!("{constructor} has no runtime constructor"),
        )),
    }
}

/// `Data(s, F)`: the text, parsed in the format the checked call names.
fn evaluate_data(
    cx: &mut dyn EvalCx,
    args: &[crate::execution::Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let [text, format] = args else {
        return Err(Diagnostic::runtime(span, "`Data` needs text and a format"));
    };
    let Value::Str(text) = cx.evaluate(&text.value)? else {
        return Err(Diagnostic::runtime(span, "`Data` parses a String"));
    };
    let parsed = crate::io::parse_text(&text, format.value.ty.constructor)
        .map_err(|message| Diagnostic::runtime(span, message))?;
    Ok(Value::Data(Arc::new(parsed)))
}
