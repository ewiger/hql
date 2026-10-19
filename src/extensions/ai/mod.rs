//! The `ai` extension: agents read from cards, run over a context, with every
//! stage of the run kept as a value.
//!
//! See `doc/proposals/HQL-0022/README.md`. The constructors (`Agent`,
//! `Context`, `Prompt`, `Tool`) are pure. `run`, `ask`, `delegate` and
//! `remember` are the first effectful steps: each happens once, where it is
//! written, and only as the host's mode allows. A run is never collapsed into
//! its answer; the answer is one field of the `Run` the harness returns.

pub mod transport;
pub(crate) mod wire;

use super::collections::wrong;
use super::spec::{ArgumentKind, ArgumentSpec, OperationSpec, Output as O, TypePattern as P};
use super::{CheckCx, EvalCx, Operation, Purity, named, named_or_first};
use crate::ast::Arg;
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{Formatted, Record, TypeConstructor, TypeRef, Value};
use serde_json::{Value as J, json};
use std::cell::Cell;
use std::ops::Range;
use std::sync::Arc;
use transport::Mode;
use wire::{Call, Message, Outcome, Stop, ToolSpec};

/// The most bytes one tool result may return to a model.
const TOOL_OUTPUT_LIMIT: usize = 65_536;
/// The most calls one response may request; later ones are refused.
const CALLS_PER_RESPONSE: usize = 16;
/// How far below the outermost run delegation through tools may reach.
const DELEGATION_DEPTH: usize = 3;
/// The most model requests a run may make, whatever its card says.
const TURN_LIMIT: i64 = 64;

thread_local! {
    /// Whether a tool's function is being evaluated: a tool is pure, so
    /// nothing a model asks for may reach an effect.
    static IN_TOOL: Cell<bool> = const { Cell::new(false) };
    /// The prose of the section the cell being evaluated sits in: the
    /// instructions a prompt is given when none are written.
    static SECTION: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Evaluate with `section` as the default instructions of every prompt built
/// meanwhile: what a cell's surrounding prose asks.
pub(crate) fn within_section<R>(section: &str, evaluate: impl FnOnce() -> R) -> R {
    let previous = SECTION.with(|held| held.replace(Some(section.to_owned())));
    let result = evaluate();
    SECTION.with(|held| *held.borrow_mut() = previous);
    result
}

/// The instructions a prompt takes when none are written: the cell's section.
fn section() -> Option<String> {
    SECTION.with(|held| held.borrow().clone())
}

const fn ty(name: &'static str) -> P {
    P::Type(TypeConstructor(name))
}

const fn optional(name: &'static str, kind: ArgumentKind) -> ArgumentSpec {
    ArgumentSpec {
        name,
        kind,
        required: false,
        positional: false,
        named: true,
    }
}

const TOOLS: ArgumentSpec = optional(
    "tools",
    ArgumentKind::Value(P::Apply(TypeConstructor("List"), &[ty("Tool")])),
);

/// Every step the `ai` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "run",
        spec: OperationSpec {
            parameters: &[],
            input: P::OneOf(&[ty("Context"), ty("Prompt")]),
            arguments: &[ArgumentSpec::value("agent", ty("Agent")), TOOLS],
            output: O::Type(ty("Run")),
        },
        summary: "Execute an agent over a context or a prompt, keeping every turn, call and event.",
        purity: Purity::Effectful,
        check: None,
        eval: eval_run,
    },
    Operation {
        name: "ask",
        spec: OperationSpec {
            parameters: &[],
            input: ty("Run"),
            arguments: &[ArgumentSpec::value("message", ty("String"))],
            output: O::Type(ty("Run")),
        },
        summary: "Continue a run's conversation with one more message, as a new run.",
        purity: Purity::Effectful,
        check: None,
        eval: eval_ask,
    },
    Operation {
        name: "delegate",
        spec: OperationSpec {
            parameters: &[],
            input: ty("Run"),
            arguments: &[
                ArgumentSpec::value("agent", ty("Agent")),
                ArgumentSpec::value("task", ty("String")),
                TOOLS,
            ],
            output: O::Type(ty("Run")),
        },
        summary: "Hand a run's context and answer to another agent, as a new run.",
        purity: Purity::Effectful,
        check: None,
        eval: eval_delegate,
    },
    Operation {
        name: "remember",
        spec: OperationSpec {
            parameters: &[],
            input: P::OneOf(&[ty("Run"), ty("String")]),
            arguments: &[ArgumentSpec::value("name", ty("String"))],
            output: O::Type(ty("Card")),
        },
        summary: "Write a run's answer, or a text, as a memory card.",
        purity: Purity::Effectful,
        check: None,
        eval: eval_remember,
    },
    Operation {
        name: "trace",
        spec: OperationSpec {
            parameters: &[],
            input: ty("Run"),
            arguments: &[],
            output: O::Type(P::Apply(TypeConstructor("List"), &[ty("Event")])),
        },
        summary: "A run's events, with each child's inserted after the call that started it.",
        purity: Purity::Pure,
        check: None,
        eval: eval_trace,
    },
];

// Constructors ---------------------------------------------------------------

/// The types this module constructs.
pub(crate) const CONSTRUCTORS: [&str; 4] = ["Agent", "Context", "Prompt", "Tool"];

fn of(name: &'static str) -> TypeRef {
    TypeRef::from(TypeConstructor(name))
}

/// The type a constructor call has, or why its arguments do not fit.
pub(crate) fn check_constructor(
    cx: &mut dyn CheckCx,
    name: &str,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let positional: Vec<&Arg> = args.iter().filter(|arg| arg.name.is_none()).collect();
    let refuse = |message: String| Err::<TypeRef, _>(Diagnostic::typing(span.clone(), message));
    let Some(first) = positional.first() else {
        return refuse(format!("{name} needs an argument"));
    };
    let first_type = cx.infer(&first.value)?;
    // The text argument a constructor takes second, positionally or by name.
    let mut text = |label: &str, required: bool| -> Result<(), Diagnostic> {
        let argument = named(args, label).or_else(|| positional.get(1).copied());
        match argument {
            None if required => refuse(format!("{name} needs `{label}`, a String")).map(drop),
            None => Ok(()),
            Some(argument) => {
                let found = cx.infer(&argument.value)?;
                if found.is(&TypeRef::STR) {
                    Ok(())
                } else {
                    refuse(format!("{name}'s `{label}` is a String, not {found}")).map(drop)
                }
            }
        }
    };
    match name {
        "Agent" => {
            if !first_type.present().is(&TypeRef::CARD) {
                return refuse(format!("an Agent is read from a card, not {first_type}"));
            }
        }
        "Context" => text("label", false)?,
        "Prompt" => {
            if !first_type.is(&of("Context")) {
                return refuse(format!(
                    "a Prompt is built from a Context, not {first_type}"
                ));
            }
            // Inside a cell, the cell's section is the default.
            text("instructions", false)?;
        }
        "Tool" => {
            if !first_type.constructor.0.starts_with("Fn") {
                return refuse(format!("a Tool is made from a function, not {first_type}"));
            }
            text("description", true)?;
        }
        _ => return refuse(format!("{name} has no constructor")),
    }
    Ok(of(match name {
        "Agent" => "Agent",
        "Context" => "Context",
        "Prompt" => "Prompt",
        _ => "Tool",
    }))
}

/// Construct a value of one of this module's types.
pub(crate) fn construct(
    cx: &mut dyn EvalCx,
    name: &str,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let positional: Vec<&TypedArg> = args.iter().filter(|arg| arg.name.is_none()).collect();
    let first = match positional.first() {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => {
            return Err(Diagnostic::runtime(
                span,
                format!("{name} needs an argument"),
            ));
        }
    };
    let mut text = |label: &str| -> Result<Option<String>, Diagnostic> {
        match named(args, label).or_else(|| positional.get(1).copied()) {
            Some(argument) => Ok(Some(string_of(&cx.evaluate(&argument.value)?))),
            None => Ok(None),
        }
    };
    match name {
        "Agent" => agent(cx, &first, &span),
        "Context" => {
            let label = text("label")?.unwrap_or_else(|| "context".to_owned());
            context(cx, first, &label, &span)
        }
        "Prompt" => {
            let instructions = match text("instructions")? {
                Some(instructions) => instructions,
                None => section().ok_or_else(|| {
                    runtime(
                        &span,
                        "a Prompt outside a cell needs `instructions`; inside one, the cell's \
                         section is the default",
                    )
                })?,
            };
            Ok(prompt(first, &instructions))
        }
        "Tool" => {
            let description = text("description")?.unwrap_or_default();
            tool(first, &description, &span)
        }
        _ => Err(Diagnostic::runtime(
            span,
            format!("{name} has no constructor"),
        )),
    }
}

fn record(name: &'static str, fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Arc::new(Record::new(of(name), fields)))
}

fn text(value: impl Into<String>) -> Value {
    Value::Str(Arc::from(value.into()))
}

fn formatted(value: impl Into<String>, format: &'static str) -> Value {
    Value::Formatted(Arc::new(Formatted {
        text: value.into(),
        format: TypeConstructor(format),
    }))
}

fn strings(values: Vec<String>) -> Value {
    Value::list(values.into_iter().map(text).collect(), TypeRef::STR)
}

fn string_of(value: &Value) -> String {
    match value {
        Value::Str(text) => text.to_string(),
        Value::Formatted(formatted) => formatted.text.clone(),
        Value::Present(inner) => string_of(inner),
        other => other.to_string(),
    }
}

fn runtime(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message.into())
}

/// `Agent(card)`: the card's body as instructions, its `agent` table as
/// configuration.
fn agent(cx: &dyn EvalCx, value: &Value, span: &Range<usize>) -> Result<Value, Diagnostic> {
    let card = match value {
        Value::Absent(_) => {
            return Err(runtime(
                span,
                "an Agent needs a card, and the link did not resolve",
            ));
        }
        other => other.as_card().ok_or_else(|| {
            runtime(
                span,
                format!("an Agent is read from a card, not {}", other.type_of()),
            )
        })?,
    };
    let name = card.name().to_owned();
    let Some(Data::Map(table)) = card.document.header.get("agent") else {
        return Err(runtime(
            span,
            format!("`{name}` is not an agent: its header has no `agent` table"),
        ));
    };
    let names = |key: &str| -> Result<Vec<String>, Diagnostic> {
        match table.get(key) {
            None => Ok(Vec::new()),
            Some(Data::List(items)) => items
                .iter()
                .map(|item| {
                    item.as_str().map(str::to_owned).ok_or_else(|| {
                        runtime(span, format!("`{name}`: `agent.{key}` lists names"))
                    })
                })
                .collect(),
            Some(_) => Err(runtime(span, format!("`{name}`: `agent.{key}` is a list"))),
        }
    };
    for key in table.keys() {
        if !matches!(key.as_str(), "model" | "tools" | "agents" | "max_turns") {
            return Err(runtime(
                span,
                format!("`{name}`: unknown key `{key}` in its `agent` table"),
            ));
        }
    }
    let model = match table.get("model") {
        None => "default".to_owned(),
        Some(model) => model
            .as_str()
            .ok_or_else(|| runtime(span, format!("`{name}`: `agent.model` names a model")))?
            .to_owned(),
    };
    let max_turns = match table.get("max_turns") {
        None => 8,
        Some(Data::Int(turns)) if (1..=TURN_LIMIT).contains(turns) => *turns,
        Some(_) => {
            return Err(runtime(
                span,
                format!("`{name}`: `agent.max_turns` is between 1 and {TURN_LIMIT}"),
            ));
        }
    };
    let _ = cx;
    Ok(record(
        "Agent",
        vec![
            ("name", text(name.clone())),
            (
                "instructions",
                formatted(card.document.body.trim(), "Markdown"),
            ),
            ("model", text(model)),
            ("tools", strings(names("tools")?)),
            ("agents", strings(names("agents")?)),
            ("max_turns", Value::Int(max_turns)),
            ("card", Value::Card(card)),
        ],
    ))
}

/// `Context(x, label)`: one item per element of a sequence, one for anything
/// else.
fn context(
    cx: &mut dyn EvalCx,
    value: Value,
    label: &str,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let values = match value {
        Value::Traversal(traversal) if traversal.forced().is_none() => {
            let mut values = Vec::new();
            cx.traverse(&traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            values
        }
        Value::Str(_) | Value::Formatted(_) => vec![value],
        other => other.elements().unwrap_or_else(|| vec![other]),
    };
    let items = values
        .into_iter()
        .map(|value| item(value, label, span))
        .collect::<Result<_, _>>()?;
    Ok(Record::context(items))
}

fn item(value: Value, label: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
    if let Value::Record(held) = &value
        && held.is("ContextItem")
    {
        return Ok(value);
    }
    let kind = value.type_of().to_string();
    let mut score = None;
    let (shown, provenance) = match &value {
        Value::Hit(hit) => {
            score = Some(hit.score);
            let retrieval = &hit.provenance;
            let mut provenance = card_provenance(&hit.value);
            provenance["retrieval"] = json!({
                "query": retrieval.query, "index": retrieval.index, "model": retrieval.model,
                "revision": retrieval.revision, "metric": retrieval.metric,
                "approximate": retrieval.approximate,
            });
            (hit.value.to_markdown(), provenance)
        }
        Value::Record(run) if run.is("Run") => {
            let result = match run.get("result") {
                Some(result) => string_of(result),
                None => {
                    return Err(runtime(
                        span,
                        "a run with no result cannot be part of a context",
                    ));
                }
            };
            let agent = match run.get("agent") {
                Some(Value::Record(agent)) => agent.text("name"),
                _ => String::new(),
            };
            (result, json!({"run": run.text("id"), "agent": agent}))
        }
        Value::Str(text) => (text.to_string(), json!({})),
        other => (other.to_markdown(), card_provenance(other)),
    };
    let mut fields = vec![
        ("label", text(label)),
        ("kind", text(kind)),
        ("text", formatted(shown, "Markdown")),
    ];
    if let Some(score) = score {
        fields.push(("score", Value::Float(score)));
    }
    fields.push(("provenance", Value::Data(Arc::new(data(&provenance)))));
    Ok(record("ContextItem", fields))
}

fn card_provenance(value: &Value) -> J {
    match value.as_card() {
        Some(card) => json!({
            "name": card.name(),
            "path": card.document.path.display().to_string(),
        }),
        None => json!({}),
    }
}

/// `Prompt(ctx, instructions)`, laid out as HQL-0022 pins it.
fn prompt(context: Value, instructions: &str) -> Value {
    let items = context.elements().unwrap_or_default();
    let mut labels: Vec<String> = Vec::new();
    for item in &items {
        if let Value::Record(item) = item {
            let label = item.text("label");
            if !labels.contains(&label) {
                labels.push(label);
            }
        }
    }
    let mut rendered = instructions.to_owned();
    for label in &labels {
        rendered.push_str(&format!("\n\n## {label}"));
        for item in &items {
            let Value::Record(item) = item else { continue };
            if item.text("label") != *label {
                continue;
            }
            rendered.push_str("\n\n");
            if let Some(Value::Data(provenance)) = item.get("provenance")
                && let Some(name) = provenance.get("name").and_then(Data::as_str)
            {
                rendered.push_str(&format!("Source: {name}\n\n"));
            }
            rendered.push_str(&item.text("text"));
        }
    }
    let rendered = rendered.trim_start().to_owned();
    record(
        "Prompt",
        vec![
            ("instructions", formatted(instructions, "Markdown")),
            ("context", context),
            ("rendered", formatted(rendered, "Markdown")),
        ],
    )
}

/// `Tool(f, description)`: a named function, with the JSON Schema of its
/// parameters.
fn tool(function: Value, description: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
    let Value::Function(closure) = &function else {
        return Err(runtime(span, "a Tool is made from a function"));
    };
    let Some(name) = closure.name.clone() else {
        return Err(runtime(
            span,
            "a Tool is made from a declared function: a lambda has no name to call it by",
        ));
    };
    let types = &closure.ty.args[..closure.ty.args.len().saturating_sub(1)];
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for (parameter, ty) in closure.parameters.iter().zip(types) {
        let schema = schema(ty).ok_or_else(|| {
            runtime(
                span,
                format!(
                    "`{name}`'s parameter `{parameter}` is a {ty}, which no JSON Schema \
                     describes; a tool takes Bool, Int, Float, String, lists, maps and Data"
                ),
            )
        })?;
        if ty.constructor.0 != "Option" {
            required.push(json!(parameter));
        }
        properties.insert(parameter.clone(), schema);
    }
    let parameters = json!({"type": "object", "properties": properties, "required": required});
    Ok(record(
        "Tool",
        vec![
            ("name", text(name)),
            ("description", text(description)),
            ("parameters", formatted(parameters.to_string(), "Json")),
            ("function", function),
        ],
    ))
}

/// The JSON Schema of a parameter type, when one describes it.
fn schema(ty: &TypeRef) -> Option<J> {
    Some(match ty.constructor.0 {
        "Bool" => json!({"type": "boolean"}),
        "Int" => json!({"type": "integer"}),
        "Float" => json!({"type": "number"}),
        "String" => json!({"type": "string"}),
        "Data" => json!({}),
        "Option" => return schema(ty.args.first()?),
        "List" => json!({"type": "array", "items": schema(ty.args.first()?)?}),
        "Map" if ty.args.first() == Some(&TypeRef::STR) => {
            json!({"type": "object", "additionalProperties": schema(ty.args.get(1)?)?})
        }
        _ => return None,
    })
}

/// A JSON value as an open tree.
fn data(value: &J) -> Data {
    match value {
        J::Null => Data::Empty,
        J::Bool(flag) => Data::Bool(*flag),
        J::Number(number) => number
            .as_i64()
            .map_or_else(|| Data::Float(number.as_f64().unwrap_or(0.0)), Data::Int),
        J::String(text) => Data::Str(text.clone()),
        J::Array(items) => Data::List(items.iter().map(data).collect()),
        J::Object(map) => Data::Map(
            map.iter()
                .map(|(key, value)| (key.clone(), data(value)))
                .collect(),
        ),
    }
}

/// A JSON argument as the HQL value a parameter of `ty` takes, or why not.
fn argument(value: &J, ty: &TypeRef) -> Result<Value, String> {
    let wrong = || format!("expected {ty}, got {value}");
    Ok(match ty.constructor.0 {
        "Bool" => Value::Bool(value.as_bool().ok_or_else(wrong)?),
        "Int" => Value::Int(value.as_i64().ok_or_else(wrong)?),
        "Float" => Value::Float(value.as_f64().ok_or_else(wrong)?),
        "String" => text(value.as_str().ok_or_else(wrong)?),
        "Data" => Value::Data(Arc::new(data(value))),
        "Option" => {
            let inner = ty.args.first().ok_or_else(wrong)?;
            if value.is_null() {
                Value::Absent(inner.clone())
            } else {
                Value::Present(Arc::new(argument(value, inner)?))
            }
        }
        "List" => {
            let element = ty.args.first().ok_or_else(wrong)?;
            let items = value.as_array().ok_or_else(wrong)?;
            Value::list(
                items
                    .iter()
                    .map(|item| argument(item, element))
                    .collect::<Result<_, _>>()?,
                element.clone(),
            )
        }
        "Map" => {
            let element = ty.args.get(1).ok_or_else(wrong)?;
            let map = value.as_object().ok_or_else(wrong)?;
            let mut keys = Vec::new();
            let mut values = Vec::new();
            for (key, item) in map {
                keys.push(text(key.clone()));
                values.push(argument(item, element)?);
            }
            let map = crate::types::MapValue::new(
                crate::types::MapKind::Unordered,
                keys,
                values,
                TypeRef::STR,
                element.clone(),
            )
            .map_err(|error| error.to_string())?;
            Value::Map(Arc::new(map))
        }
        _ => return Err(wrong()),
    })
}

// The harness ------------------------------------------------------------------

/// What one execution starts from.
struct Start {
    agent: Arc<Record>,
    prompt: Value,
    messages: Vec<Message>,
    tools: Vec<Arc<Record>>,
    origin: &'static str,
    parent: Option<Value>,
    depth: usize,
}

fn refuse_inside_a_tool(step: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
    if IN_TOOL.with(Cell::get) {
        return Err(runtime(
            span,
            format!("a tool is pure, and `{step}` is effectful"),
        ));
    }
    Ok(())
}

fn as_record(value: &Value, name: &str, span: &Range<usize>) -> Result<Arc<Record>, Diagnostic> {
    match value {
        Value::Record(record) if record.is(name) => Ok(Arc::clone(record)),
        Value::Present(inner) => as_record(inner, name, span),
        other => Err(runtime(
            span,
            format!("expected {name}, not {}", other.type_of()),
        )),
    }
}

fn tools_argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    span: &Range<usize>,
) -> Result<Vec<Arc<Record>>, Diagnostic> {
    let Some(argument) = named(arguments, "tools") else {
        return Ok(Vec::new());
    };
    cx.evaluate(&argument.value)?
        .elements()
        .unwrap_or_default()
        .iter()
        .map(|tool| as_record(tool, "Tool", span))
        .collect()
}

fn agent_argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    span: &Range<usize>,
) -> Result<Arc<Record>, Diagnostic> {
    let argument =
        named_or_first(arguments, "agent").ok_or_else(|| wrong("run", "an agent", span))?;
    match cx.evaluate(&argument.value)? {
        Value::Absent(_) => Err(runtime(span, "the agent's card did not resolve")),
        value => as_record(&value, "Agent", span),
    }
}

fn eval_run(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    refuse_inside_a_tool("run", &span)?;
    let agent = agent_argument(cx, arguments, &span)?;
    let tools = tools_argument(cx, arguments, &span)?;
    let prompt = match &input {
        Value::Record(record) if record.is("Prompt") => input.clone(),
        _ => prompt(input, &section().unwrap_or_default()),
    };
    let rendered = match &prompt {
        Value::Record(record) => record.text("rendered"),
        _ => String::new(),
    };
    execute(
        cx,
        Start {
            agent,
            prompt,
            messages: vec![Message::User(rendered)],
            tools,
            origin: "run",
            parent: None,
            depth: 0,
        },
        &span,
    )
}

fn eval_ask(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    refuse_inside_a_tool("ask", &span)?;
    let run = as_record(&input, "Run", &span)?;
    let argument =
        named_or_first(arguments, "message").ok_or_else(|| wrong("ask", "a message", &span))?;
    let message = string_of(&cx.evaluate(&argument.value)?);
    let mut messages = conversation(&run);
    messages.push(Message::User(message));
    execute(
        cx,
        Start {
            agent: as_record(run.get("agent").unwrap_or(&Value::Unit), "Agent", &span)?,
            prompt: run.get("prompt").cloned().unwrap_or(Value::Unit),
            messages,
            tools: run
                .list("tools")
                .iter()
                .map(|tool| as_record(tool, "Tool", &span))
                .collect::<Result<_, _>>()?,
            origin: "ask",
            parent: Some(input),
            depth: 0,
        },
        &span,
    )
}

fn eval_delegate(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    refuse_inside_a_tool("delegate", &span)?;
    let run = as_record(&input, "Run", &span)?;
    let agent = agent_argument(cx, arguments, &span)?;
    let tools = tools_argument(cx, arguments, &span)?;
    let task = match named(arguments, "task")
        .or_else(|| arguments.iter().filter(|arg| arg.name.is_none()).nth(1))
    {
        Some(argument) => string_of(&cx.evaluate(&argument.value)?),
        None => return Err(wrong("delegate", "a task", &span)),
    };
    let earlier = match run.get("prompt") {
        Some(Value::Record(prompt)) => prompt.list("items"),
        _ => Vec::new(),
    };
    let earlier = match run.get("prompt") {
        Some(Value::Record(prompt)) => match prompt.get("context") {
            Some(context) => context.elements().unwrap_or_default(),
            None => earlier,
        },
        _ => earlier,
    };
    let mut items = earlier;
    items.push(item(input.clone(), "delegated", &span)?);
    let prompt = prompt(Record::context(items), &task);
    let rendered = match &prompt {
        Value::Record(record) => record.text("rendered"),
        _ => String::new(),
    };
    execute(
        cx,
        Start {
            agent,
            prompt,
            messages: vec![Message::User(rendered)],
            tools,
            origin: "delegate",
            parent: Some(input),
            depth: 0,
        },
        &span,
    )
}

/// A run's conversation, read back out of its turns and calls.
fn conversation(run: &Record) -> Vec<Message> {
    let calls: Vec<Arc<Record>> = run
        .list("calls")
        .into_iter()
        .filter_map(|call| match call {
            Value::Record(call) => Some(call),
            _ => None,
        })
        .collect();
    let of_turn = |turn: i64| {
        calls
            .iter()
            .filter(move |call| matches!(call.get("turn"), Some(Value::Int(at)) if *at == turn))
    };
    let mut messages = Vec::new();
    let mut last_assistant = 0;
    for (index, turn) in run.list("turns").into_iter().enumerate() {
        let Value::Record(turn) = turn else { continue };
        let index = i64::try_from(index).unwrap_or_default();
        match turn.text("role").as_str() {
            "user" => messages.push(Message::User(turn.text("text"))),
            "assistant" => {
                last_assistant = index;
                messages.push(Message::Assistant {
                    text: turn.text("text"),
                    calls: of_turn(index)
                        .map(|call| Call {
                            id: call.text("id"),
                            tool: call.text("tool"),
                            arguments: call.get("arguments").map_or(json!({}), Value::to_json),
                        })
                        .collect(),
                });
            }
            _ => messages.push(Message::Results(
                of_turn(last_assistant)
                    .map(|call| Outcome {
                        id: call.text("id"),
                        output: call.text("output"),
                        failed: call.text("status") != "ok",
                    })
                    .collect(),
            )),
        }
    }
    messages
}

/// Everything one execution accumulates.
#[derive(Default)]
struct Log {
    turns: Vec<Value>,
    calls: Vec<Value>,
    children: Vec<Value>,
    events: Vec<Value>,
    id: String,
}

impl Log {
    fn event(&mut self, kind: &str, payload: J) {
        let seq = i64::try_from(self.events.len()).unwrap_or_default();
        let event = record(
            "Event",
            vec![
                ("kind", text(kind)),
                ("run", text(self.id.clone())),
                ("seq", Value::Int(seq)),
                ("data", Value::Data(Arc::new(data(&payload)))),
            ],
        );
        self.events.push(event);
    }

    fn turn(&mut self, role: &str, said: impl Into<String>) -> i64 {
        self.turns.push(record(
            "Turn",
            vec![("role", text(role)), ("text", text(said))],
        ));
        i64::try_from(self.turns.len() - 1).unwrap_or_default()
    }
}

fn execute(cx: &mut dyn EvalCx, start: Start, span: &Range<usize>) -> Result<Value, Diagnostic> {
    let vault = cx.vault();
    let root = vault.root.clone();
    let settings = vault.ai.clone();
    let model = settings
        .model(&start.agent.text("model"))
        .map_err(|message| runtime(span, message))?
        .clone();
    let wanted = start.agent.list("tools");
    let wanted: Vec<String> = wanted.iter().map(string_of).collect();
    let given: Vec<String> = start.tools.iter().map(|tool| tool.text("name")).collect();
    for name in &wanted {
        if !given.contains(name) {
            return Err(runtime(
                span,
                format!(
                    "`{}` lists the tool `{name}`, and the run was not given it",
                    start.agent.text("name")
                ),
            ));
        }
    }
    for name in &given {
        if !wanted.contains(name) {
            return Err(runtime(
                span,
                format!(
                    "the run was given `{name}`, which `{}` does not list: its card is where \
                     what it can do is written",
                    start.agent.text("name")
                ),
            ));
        }
    }
    let delegates: Vec<String> = start.agent.list("agents").iter().map(string_of).collect();
    let mut specs: Vec<ToolSpec> = start
        .tools
        .iter()
        .map(|tool| ToolSpec {
            name: tool.text("name"),
            description: tool.text("description"),
            schema: serde_json::from_str(&tool.text("parameters")).unwrap_or(json!({})),
        })
        .collect();
    for name in &delegates {
        specs.push(ToolSpec {
            name: name.clone(),
            description: format!("Delegate a task to the agent `{name}`, and receive its answer."),
            schema: json!({"type": "object", "properties": {"task": {"type": "string"}}, "required": ["task"]}),
        });
    }
    let system = start.agent.text("instructions");
    let mut messages = start.messages;
    let mut log = Log::default();
    for message in &messages {
        match message {
            Message::User(said) => {
                log.turn("user", said.clone());
            }
            Message::Assistant { text, .. } => {
                log.turn("assistant", text.clone());
            }
            Message::Results(outcomes) => {
                let said: Vec<&str> = outcomes
                    .iter()
                    .map(|outcome| outcome.output.as_str())
                    .collect();
                log.turn("tool", said.join("\n"));
            }
        }
    }
    // The calls of a continued conversation stay with the run they belong to.
    let mut stop = "max_turns";
    let mut result = None;
    let mut reported = json!({});
    let mut usage = json!({});
    let mut body = J::Null;
    let max_turns = match start.agent.get("max_turns") {
        Some(Value::Int(turns)) => *turns,
        _ => 8,
    };
    for _ in 0..max_turns {
        body = wire::request(&model, &system, &messages, &specs);
        let key = transport::key(&body);
        if log.id.is_empty() {
            log.id.clone_from(&key);
            log.event(
                "prompt",
                json!({"origin": start.origin, "agent": start.agent.text("name")}),
            );
        }
        log.event("request", json!({"key": key}));
        let response = transport::send(&root, &settings, &model, &body)
            .map_err(|message| runtime(span, message))?;
        let reply = wire::reply(model.api, &response).map_err(|message| runtime(span, message))?;
        reported =
            json!({"api": model.api.name(), "endpoint": model.endpoint, "model": reply.model});
        add_usage(&mut usage, &reply.usage);
        log.event(
            "response",
            json!({"stop": format!("{:?}", reply.stop).to_lowercase(), "usage": reply.usage}),
        );
        let turn = log.turn("assistant", reply.text.clone());
        messages.push(Message::Assistant {
            text: reply.text.clone(),
            calls: reply.calls.clone(),
        });
        match reply.stop {
            Stop::Refused => {
                stop = "refused";
                break;
            }
            Stop::End | Stop::MaxTokens if reply.calls.is_empty() => {
                stop = if reply.stop == Stop::End {
                    "end"
                } else {
                    "max_tokens"
                };
                result = Some(reply.text);
                break;
            }
            _ => {}
        }
        let mut outcomes = Vec::new();
        for (index, call) in reply.calls.iter().enumerate() {
            let (status, output, child) = if index >= CALLS_PER_RESPONSE {
                (
                    "refused",
                    format!("at most {CALLS_PER_RESPONSE} calls per response"),
                    None,
                )
            } else if delegates.contains(&call.tool) {
                delegate_through_tool(cx, &start.tools, call, start.depth, span)?
            } else {
                let (status, output) = call_tool(cx, &start.tools, call, span);
                (status, output, None)
            };
            let mut payload = json!({"tool": call.tool, "id": call.id, "status": status});
            if let Some(Value::Record(child)) = &child {
                payload["child"] = json!(child.text("id"));
            }
            log.event("tool_call", payload);
            if let Some(child) = child {
                log.children.push(child);
            }
            log.calls.push(record(
                "ToolCall",
                vec![
                    ("id", text(call.id.clone())),
                    ("tool", text(call.tool.clone())),
                    ("arguments", Value::Data(Arc::new(data(&call.arguments)))),
                    ("status", text(status)),
                    ("output", formatted(output.clone(), "Json")),
                    ("turn", Value::Int(turn)),
                ],
            ));
            outcomes.push(Outcome {
                id: call.id.clone(),
                output,
                failed: status != "ok",
            });
        }
        let said: Vec<&str> = outcomes
            .iter()
            .map(|outcome| outcome.output.as_str())
            .collect();
        log.turn("tool", said.join("\n"));
        messages.push(Message::Results(outcomes));
    }
    log.event("stop", json!({"stop": stop}));
    if stop == "max_turns" {
        cx.warn(crate::warnings::Warning::new(
            span.clone(),
            format!(
                "`{}` stopped after {max_turns} requests without an answer",
                start.agent.text("name")
            ),
        ));
    }
    let mut fields = vec![
        ("id", text(log.id.clone())),
        ("origin", text(start.origin)),
        ("agent", Value::Record(Arc::clone(&start.agent))),
        ("prompt", start.prompt),
    ];
    if let Some(parent) = start.parent {
        fields.push(("parent", parent));
    }
    fields.extend([
        ("turns", Value::list(log.turns, of("Turn"))),
        ("calls", Value::list(log.calls, of("ToolCall"))),
        ("children", Value::list(log.children, of("Run"))),
    ]);
    if let Some(result) = result {
        fields.push(("result", text(result)));
    }
    fields.extend([
        ("stop", text(stop)),
        ("model", Value::Data(Arc::new(data(&reported)))),
        ("usage", Value::Data(Arc::new(data(&usage)))),
        ("request", formatted(transport::canonical(&body), "Json")),
        ("events", Value::list(log.events, of("Event"))),
        (
            "tools",
            Value::list(
                start.tools.into_iter().map(Value::Record).collect(),
                of("Tool"),
            ),
        ),
    ]);
    Ok(record("Run", fields))
}

fn add_usage(total: &mut J, usage: &J) {
    let (J::Object(total), J::Object(usage)) = (total, usage) else {
        return;
    };
    for (key, value) in usage {
        if let Some(count) = value.as_i64() {
            let held = total.get(key).and_then(J::as_i64).unwrap_or(0);
            total.insert(key.clone(), json!(held + count));
        }
    }
}

/// Evaluate one requested call of an HQL function. A failure is an outcome
/// the model receives, never a failure of the run.
fn call_tool(
    cx: &mut dyn EvalCx,
    tools: &[Arc<Record>],
    call: &Call,
    span: &Range<usize>,
) -> (&'static str, String) {
    let Some(tool) = tools.iter().find(|tool| tool.text("name") == call.tool) else {
        return ("refused", format!("no tool `{}`", call.tool));
    };
    let Some(Value::Function(closure)) = tool.get("function") else {
        return ("refused", format!("`{}` has no function", call.tool));
    };
    let types = &closure.ty.args[..closure.ty.args.len().saturating_sub(1)];
    let mut values = Vec::new();
    for (parameter, ty) in closure.parameters.iter().zip(types) {
        let given = call.arguments.get(parameter).unwrap_or(&J::Null);
        if given.is_null() && ty.constructor.0 != "Option" {
            return ("refused", format!("missing argument `{parameter}`"));
        }
        match argument(given, ty) {
            Ok(value) => values.push(value),
            Err(reason) => return ("refused", format!("argument `{parameter}`: {reason}")),
        }
    }
    let function = Value::Function(Arc::clone(closure));
    IN_TOOL.with(|flag| flag.set(true));
    let returned = cx.call_function(&function, values, span);
    IN_TOOL.with(|flag| flag.set(false));
    match returned {
        Err(diagnostic) => ("failed", diagnostic.message()),
        Ok(value) => {
            let value = value.settled().unwrap_or(value);
            let output = value.to_json().to_string();
            if output.len() > TOOL_OUTPUT_LIMIT {
                (
                    "failed",
                    format!(
                        "the result is {} bytes, and a tool returns at most {TOOL_OUTPUT_LIMIT}",
                        output.len()
                    ),
                )
            } else {
                ("ok", output)
            }
        }
    }
}

/// A call to an agent named in `agents`: a child run of that agent, given
/// the tools of its parent that its card lists.
fn delegate_through_tool(
    cx: &mut dyn EvalCx,
    tools: &[Arc<Record>],
    call: &Call,
    depth: usize,
    span: &Range<usize>,
) -> Result<(&'static str, String, Option<Value>), Diagnostic> {
    if depth + 1 > DELEGATION_DEPTH {
        return Ok((
            "refused",
            format!("delegation reaches at most {DELEGATION_DEPTH} runs deep"),
            None,
        ));
    }
    let Some(card) = cx.vault().resolve(&call.tool) else {
        return Ok(("refused", format!("no card `{}`", call.tool), None));
    };
    let child = match agent(cx, &Value::Card(card), span) {
        Ok(Value::Record(child)) => child,
        Ok(_) => return Ok(("refused", format!("`{}` is not an agent", call.tool), None)),
        Err(diagnostic) => return Ok(("refused", diagnostic.message(), None)),
    };
    let wanted: Vec<String> = child.list("tools").iter().map(string_of).collect();
    let given: Vec<Arc<Record>> = tools
        .iter()
        .filter(|tool| wanted.contains(&tool.text("name")))
        .cloned()
        .collect();
    if given.len() != wanted.len() {
        return Ok((
            "refused",
            format!("`{}` needs tools its parent was not given", call.tool),
            None,
        ));
    }
    let task = call
        .arguments
        .get("task")
        .and_then(J::as_str)
        .unwrap_or_default()
        .to_owned();
    let prompt = prompt(Record::context(Vec::new()), &task);
    let run = execute(
        cx,
        Start {
            agent: child,
            prompt,
            messages: vec![Message::User(task)],
            tools: given,
            origin: "tool",
            parent: None,
            depth: depth + 1,
        },
        span,
    )?;
    let answer = match &run {
        Value::Record(run) => run.text("result"),
        _ => String::new(),
    };
    Ok(("ok", json!(answer).to_string(), Some(run)))
}

fn eval_remember(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    refuse_inside_a_tool("remember", &span)?;
    let argument =
        named_or_first(arguments, "name").ok_or_else(|| wrong("remember", "a name", &span))?;
    let name = string_of(&cx.evaluate(&argument.value)?);
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return Err(runtime(&span, format!("`{name}` is not a card name")));
    }
    let vault = cx.vault();
    let Some(directory) = vault.ai.memory.clone() else {
        return Err(runtime(
            &span,
            "`remember` writes to the directory [ai.memory] dir names, and hql.toml names none",
        ));
    };
    if directory.is_absolute() || directory.components().any(|part| part.as_os_str() == "..") {
        return Err(runtime(
            &span,
            "[ai.memory] dir must resolve inside the vault",
        ));
    }
    let content = match &input {
        Value::Record(run) if run.is("Run") => {
            let Some(result) = run.get("result").map(string_of) else {
                return Err(runtime(
                    &span,
                    "a run with no result has nothing to remember",
                ));
            };
            let agent = match run.get("agent") {
                Some(Value::Record(agent)) => agent.text("name"),
                _ => String::new(),
            };
            let model = match run.get("model") {
                Some(Value::Data(model)) => model
                    .get("model")
                    .and_then(Data::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                _ => String::new(),
            };
            format!(
                "---\nmemory:\n  run: {}\n  agent: {agent}\n  model: {model}\n---\n{}\n",
                run.text("id"),
                result.trim_end()
            )
        }
        other => format!("{}\n", string_of(other).trim_end()),
    };
    let relative = directory.join(format!("{name}.hmd"));
    let path = vault.root.join(&relative);
    let mode = vault.ai.mode;
    match std::fs::read_to_string(&path) {
        Ok(held) if held == content => {}
        Ok(_) => {
            return Err(runtime(
                &span,
                format!(
                    "{} already holds other content; memory is not rewritten",
                    relative.display()
                ),
            ));
        }
        Err(_) if mode == Mode::Replay => {
            return Err(runtime(
                &span,
                format!(
                    "{} does not exist, and --ai replay writes nothing",
                    relative.display()
                ),
            ));
        }
        Err(_) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| runtime(&span, error.to_string()))?;
            }
            std::fs::write(&path, &content).map_err(|error| runtime(&span, error.to_string()))?;
        }
    }
    let card = vault
        .card_of(&relative, &content)
        .ok_or_else(|| runtime(&span, "the memory card could not be read back"))?;
    Ok(Value::Card(Arc::new(card)))
}

fn eval_trace(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let run = as_record(&input, "Run", &span)?;
    Ok(Value::list(trace(&run), of("Event")))
}

fn trace(run: &Record) -> Vec<Value> {
    let children = run.list("children");
    let mut events = Vec::new();
    for event in run.list("events") {
        let child = match &event {
            Value::Record(event) if event.text("kind") == "tool_call" => match event.get("data") {
                Some(Value::Data(payload)) => payload
                    .get("child")
                    .and_then(Data::as_str)
                    .map(str::to_owned),
                _ => None,
            },
            _ => None,
        };
        events.push(event);
        if let Some(child) = child
            && let Some(Value::Record(found)) = children
                .iter()
                .find(|held| matches!(held, Value::Record(held) if held.text("id") == child))
        {
            events.extend(trace(found));
        }
    }
    events
}
