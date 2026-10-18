//! The `format` extension: writing a value as text in a presentation format.
//!
//! A presentation format is a subtype of `String` with a known syntax:
//! `Markdown`, `Json`, `D2` and the rest. `format(x, F)` writes any value as
//! text of format `F`, and the result is ordinary text a later step may read.
//! Which renderer writes a type in a format is chosen per vault in `hql.toml`,
//! over built-in defaults. See [HQL-0017](../../doc/proposals/HQL-0017/README.md).

use super::spec::{
    ArgumentKind as A, ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern,
    TypePattern as P,
};
use super::{EvalCx, Operation, Purity, named};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::traversal::{Source, Traversal};
use crate::types::{Formatted, TypeConstructor, TypeRef, Value};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

/// The presentation formats, each a subtype of `String`, with the name a
/// configuration and `--to` write it in.
pub const FORMATS: [(&str, &str); 8] = [
    ("Text", "text"),
    ("Markdown", "markdown"),
    ("Hmd", "hmd"),
    ("Json", "json"),
    ("Yaml", "yaml"),
    ("Csv", "csv"),
    ("D2", "d2"),
    ("Mermaid", "mermaid"),
];

/// Every renderer this binary provides, by the name a configuration uses,
/// with the formats it writes.
const RENDERERS: [(&str, &[&str]); 9] = [
    ("adjacency", &["Text"]),
    ("node-link", &["Json"]),
    ("d2", &["D2", "Markdown", "Hmd"]),
    ("mermaid", &["Mermaid", "Markdown", "Hmd"]),
    ("aligned", &["Text"]),
    ("pipe", &["Markdown", "Hmd"]),
    ("rows", &["Csv"]),
    ("records", &["Json"]),
    ("list", &["Markdown", "Hmd"]),
];

/// Graph renderers write only a `Graph`.
const GRAPH_RENDERERS: [&str; 4] = ["adjacency", "node-link", "d2", "mermaid"];

/// Whether some renderer writes a value of type `input` as `format`: a graph
/// in any format, text of one format as itself, as `Text`, or fenced in
/// `Markdown` and `Hmd`, and anything else in every format but `D2` and
/// `Mermaid`, which describe graphs.
///
/// # Errors
/// Names the type and the format.
pub fn writable(input: &TypeRef, format: &str) -> Result<(), String> {
    let input = input.present();
    // A stream that may not end is written piece by piece, which a format
    // that describes a whole cannot do.
    if input.is_stream() && !input.is_collection() {
        if matches!(format, "D2" | "Mermaid") {
            return Err(format!(
                "{format} describes a whole, and {input} may not end; bound it with `take(n)` \
                 or batch it with `collect(n)`"
            ));
        }
        return Ok(());
    }
    let from = input.constructor.0;
    let refused = || Err(format!("no renderer writes {input} as {format}"));
    if FORMATS.iter().any(|(name, _)| *name == from) {
        return match (from, format) {
            (from, to) if from == to => Ok(()),
            (_, "Text" | "Markdown" | "Hmd") => Ok(()),
            _ => refused(),
        };
    }
    if matches!(format, "D2" | "Mermaid") && !input.is(&TypeRef::GRAPH) && *input != TypeRef::DATA {
        return refused();
    }
    Ok(())
}

/// The format type a lowercase name denotes, `markdown` for `Markdown`.
#[must_use]
pub fn format_named(name: &str) -> Option<TypeConstructor> {
    FORMATS
        .iter()
        .find(|(_, written)| *written == name)
        .map(|(format, _)| TypeConstructor(format))
}

/// Which renderer a vault chose for a type in a format, read from `hql.toml`:
///
/// ```toml
/// [format.Graph]
/// markdown = "mermaid"
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Renderers {
    chosen: BTreeMap<(String, String), String>,
}

impl Renderers {
    /// Read `[format]` from a vault's configuration file.
    ///
    /// # Errors
    /// Names a renderer or format this binary does not have, so a mistake is
    /// reported when the configuration is read rather than when a query
    /// reaches it.
    pub fn read(root: &Path) -> Result<Self, String> {
        let Ok(text) = std::fs::read_to_string(root.join(crate::reporting::CONFIG_FILE)) else {
            return Ok(Self::default());
        };
        let Ok(parsed) = toml::from_str::<toml::Value>(&text) else {
            return Ok(Self::default());
        };
        let Some(section) = parsed.get("format").and_then(toml::Value::as_table) else {
            return Ok(Self::default());
        };
        let mut chosen = BTreeMap::new();
        for (ty, formats) in section {
            let Some(formats) = formats.as_table() else {
                return Err(format!("`[format.{ty}]` maps formats to renderers"));
            };
            for (written, renderer) in formats {
                let format = format_named(written).ok_or_else(|| {
                    format!("`[format.{ty}]` names the unknown format `{written}`")
                })?;
                let renderer = renderer
                    .as_str()
                    .ok_or_else(|| format!("`[format.{ty}] {written}` names a renderer"))?;
                let Some((_, writes)) = RENDERERS.iter().find(|(name, _)| *name == renderer) else {
                    return Err(format!(
                        "`[format.{ty}] {written}` names the unknown renderer `{renderer}`; \
                         the renderers are {}",
                        RENDERERS
                            .iter()
                            .map(|(name, _)| format!("`{name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                };
                if !writes.contains(&format.0) {
                    return Err(format!(
                        "the renderer `{renderer}` does not write {}",
                        format.0
                    ));
                }
                if GRAPH_RENDERERS.contains(&renderer) && ty != "Graph" {
                    return Err(format!(
                        "the renderer `{renderer}` writes a Graph, not {ty}"
                    ));
                }
                chosen.insert((ty.clone(), format.0.to_owned()), renderer.to_owned());
            }
        }
        Ok(Self { chosen })
    }

    /// The renderer configured for a value's type or its nearest supertype.
    fn configured(&self, value: &Value, format: &str) -> Option<&str> {
        let ty = value.type_of();
        self.chosen
            .iter()
            .find(|((name, written), _)| {
                written == format
                    && crate::types::named(name, &[]).is_some_and(|declared| {
                        ty.constructor == declared.constructor || ty.is(&declared)
                    })
            })
            .map(|(_, renderer)| renderer.as_str())
    }
}

/// Write a value as text of a format.
///
/// # Errors
/// Names the type and the format when no renderer writes that type in it.
pub fn render(
    value: &Value,
    format: TypeConstructor,
    limit: usize,
    renderer: Option<&str>,
    renderers: &Renderers,
) -> Result<String, String> {
    let (value, cut) = capped(value, limit);
    writable(&value.type_of(), format.0)?;
    // A value already in a format: the identity, a widening, plain text, or a
    // fence.
    if let Value::Formatted(formatted) = &value {
        return Ok(match (formatted.format.0, format.0) {
            ("Markdown" | "Hmd", "Markdown" | "Hmd") | (_, "Text") => formatted.text.clone(),
            (from, to) if from == to => formatted.text.clone(),
            _ => value.to_markdown(),
        });
    }
    let chosen = renderer
        .map(str::to_owned)
        .or_else(|| renderers.configured(&value, format.0).map(str::to_owned));
    let mut text = match chosen.as_deref() {
        Some(name) => by_name(name, &value, format)?,
        None => default(&value, format)?,
    };
    if cut > 0 {
        text.push_str(&cut_marker(format, cut));
    }
    Ok(text)
}

/// One value of a stream written as one piece of `format`: a line of text, a
/// Markdown list item, a CSV row (the header with the first), a JSON value on
/// one line, or a YAML document.
///
/// # Errors
/// Names a format that describes a whole and so has no pieces.
pub fn piece(value: &Value, format: TypeConstructor, first: bool) -> Result<String, String> {
    if let Value::Formatted(_) = value {
        return render(value, format, 0, None, &Renderers::default());
    }
    let one = Value::list(vec![value.clone()], value.type_of());
    match format.0 {
        "Text" | "String" => Ok(text(value)),
        "Markdown" | "Hmd" => Ok(one.to_markdown()),
        "Json" => serde_json::to_string(&value.to_json()).map_err(|error| error.to_string()),
        "Yaml" => serde_yaml_ng::to_string(&value.to_json())
            .map(|document| format!("---\n{}", document.trim_end()))
            .map_err(|error| error.to_string()),
        "Csv" => {
            let rows = one.to_csv();
            Ok(if first {
                rows.trim_end().to_owned()
            } else {
                rows.lines().skip(1).collect::<Vec<_>>().join("\n")
            })
        }
        other => Err(format!(
            "{other} describes a whole, and a stream may not end; bound it with `take(n)`"
        )),
    }
}

/// The built-in choice for a type in a format.
fn default(value: &Value, format: TypeConstructor) -> Result<String, String> {
    let graph = matches!(unwrapped(value), Value::Graph(_));
    match format.0 {
        "Text" | "String" if graph => by_name("adjacency", value, format),
        "Json" if graph => by_name("node-link", value, format),
        "Markdown" | "Hmd" | "D2" if graph => by_name("d2", value, format),
        "Mermaid" if graph => by_name("mermaid", value, format),
        "Text" | "String" => Ok(text(value)),
        "Markdown" | "Hmd" => Ok(value.to_markdown()),
        "Json" => Ok(json(value)),
        "Yaml" => serde_yaml_ng::to_string(&value.to_json()).map_err(|error| error.to_string()),
        "Csv" => Ok(value.to_csv()),
        other => Err(format!("no renderer writes {} as {other}", value.type_of())),
    }
}

fn by_name(name: &str, value: &Value, format: TypeConstructor) -> Result<String, String> {
    let refused = || {
        format!(
            "the renderer `{name}` does not write {} as {}",
            value.type_of(),
            format.0
        )
    };
    let fenced = |language: &str, body: String| match format.0 {
        "Markdown" | "Hmd" => format!("```{language}\n{}\n```", body.trim_end()),
        _ => body,
    };
    match (name, unwrapped(value)) {
        ("adjacency", Value::Graph(graph)) => Ok(adjacency(graph)),
        ("node-link", Value::Graph(graph)) => Ok(node_link(graph)),
        ("d2", Value::Graph(graph)) => Ok(fenced("d2", d2(graph))),
        ("mermaid", Value::Graph(graph)) => Ok(fenced("mermaid", mermaid(graph))),
        ("aligned", _) => Ok(value.to_table()),
        ("pipe", _) => Ok(value.to_markdown_table()),
        ("rows", _) => Ok(value.to_csv()),
        ("records", _) => Ok(json(value)),
        ("list", _) => Ok(value.to_markdown()),
        ("adjacency" | "node-link" | "d2" | "mermaid", _) => Err(refused()),
        _ => Err(format!("no renderer is named `{name}`")),
    }
}

/// Plain text: the value's own rendering.
fn text(value: &Value) -> String {
    value.to_string()
}

fn json(value: &Value) -> String {
    serde_json::to_string_pretty(&value.to_json()).unwrap_or_else(|_| "null".to_owned())
}

/// Each source and target joined by at least one edge, once, in edge order:
/// a drawing that does not show an edge's kind shows two kinds as one line.
fn pairs(graph: &crate::graph::Graph) -> Vec<(String, String)> {
    let mut seen = std::collections::BTreeSet::new();
    graph
        .induced()
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .filter(|pair| seen.insert(pair.clone()))
        .collect()
}

fn adjacency(graph: &crate::graph::Graph) -> String {
    let edges = pairs(graph);
    graph
        .nodes
        .iter()
        .map(|node| {
            // A node that was not simply a member says why it is present, so
            // the drawing does not claim that every node matched.
            let mut lines = vec![match graph.presence.get(node) {
                Some(presence) if *presence != crate::graph::Presence::Member => {
                    format!("{node}  ({})", presence.label())
                }
                _ => node.clone(),
            }];
            lines.extend(
                edges
                    .iter()
                    .filter(|(source, _)| source == node)
                    .map(|(_, target)| format!("  -> {target}")),
            );
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn node_link(graph: &crate::graph::Graph) -> String {
    let document = serde_json::json!({
        "nodes": graph.nodes.iter().map(|node| serde_json::json!({"data": {
            "id": node,
            "presence": graph.presence.get(node).map(crate::graph::Presence::label),
        }})).collect::<Vec<_>>(),
        "edges": graph.induced().iter().map(|edge| serde_json::json!({
            "data": {"source": edge.source, "target": edge.target},
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "null".to_owned())
}

fn d2(graph: &crate::graph::Graph) -> String {
    let edges = pairs(graph);
    let mut lines: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| {
            !edges
                .iter()
                .any(|(source, target)| source == *node || target == *node)
        })
        .cloned()
        .collect();
    lines.extend(
        edges
            .iter()
            .map(|(source, target)| format!("{source} -> {target}")),
    );
    lines.join("\n")
}

fn mermaid(graph: &crate::graph::Graph) -> String {
    // A Mermaid node id is a plain word, so each node is numbered and labelled.
    let ids: BTreeMap<&str, String> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.as_str(), format!("n{index}")))
        .collect();
    let mut lines = vec!["graph LR".to_owned()];
    lines.extend(
        graph
            .nodes
            .iter()
            .map(|node| format!("  {}[\"{}\"]", ids[node.as_str()], node.replace('"', "'"))),
    );
    for (source, target) in pairs(graph) {
        if let (Some(source), Some(target)) = (ids.get(source.as_str()), ids.get(target.as_str())) {
            lines.push(format!("  {source} --> {target}"));
        }
    }
    lines.join("\n")
}

/// The value with at most `limit` top-level elements, and how many were cut.
fn capped(value: &Value, limit: usize) -> (Value, usize) {
    if limit == 0 {
        return (value.clone(), 0);
    }
    match value {
        Value::List(values, element) if values.len() > limit => (
            Value::list(values[..limit].to_vec(), element.clone()),
            values.len() - limit,
        ),
        Value::Set(values, element) if values.len() > limit => (
            Value::set(values[..limit].to_vec(), element.clone()),
            values.len() - limit,
        ),
        Value::Ranking(ranking) if ranking.hits.len() > limit => {
            let mut kept = ranking.as_ref().clone();
            kept.hits.truncate(limit);
            (Value::Ranking(Arc::new(kept)), ranking.hits.len() - limit)
        }
        Value::Graph(graph) if graph.nodes.len() > limit => {
            let kept: std::collections::BTreeSet<&String> =
                graph.nodes.iter().take(limit).collect();
            let presence = graph
                .presence
                .iter()
                .filter(|(node, _)| kept.contains(node))
                .map(|(node, presence)| (node.clone(), presence.clone()))
                .collect();
            let edges = graph
                .edges
                .iter()
                .filter(|edge| kept.contains(&edge.source) && kept.contains(&edge.target))
                .cloned()
                .collect();
            (
                Value::Graph(Arc::new(crate::graph::Graph::new(presence, edges))),
                graph.nodes.len() - limit,
            )
        }
        other => (other.clone(), 0),
    }
}

/// How a format says some elements were left out.
fn cut_marker(format: TypeConstructor, cut: usize) -> String {
    match format.0 {
        "Json" | "Csv" => String::new(),
        "Yaml" | "D2" | "Mermaid" => format!("\n# {cut} more not shown"),
        _ => format!("\n… {cut} more not shown"),
    }
}

/// Every operation the `format` extension provides: the longhand and one
/// shorthand per format.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "format",
        spec: OperationSpec {
            parameters: &[TypeParameter {
                name: "F",
                bound: Some(TypeConstructor("String")),
            }],
            input: P::Any,
            arguments: &[
                ArgumentSpec {
                    name: "as",
                    kind: A::TypeName("F"),
                    required: true,
                    positional: true,
                    named: true,
                },
                ArgumentSpec {
                    required: false,
                    positional: false,
                    ..ArgumentSpec::value("limit", P::Type(TypeConstructor("Int")))
                },
                ArgumentSpec {
                    required: false,
                    positional: false,
                    ..ArgumentSpec::value("renderer", P::Type(TypeConstructor("String")))
                },
            ],
            output: O::Finite {
                finite: P::Var("F"),
                open: P::Apply(crate::types::collections::STREAM, &[P::Var("F")]),
            },
        },
        summary: "Write any value as text of a presentation format, chosen by its type; \
                  a stream piece by piece.",
        purity: Purity::Pure,
        check: Some(check_format),
        eval: eval_format,
    },
    shorthand(
        "text",
        "Text",
        &[P::Type(TypeConstructor("Text"))],
        check_text,
        eval_text,
    ),
    shorthand(
        "markdown",
        "Markdown",
        &[P::Type(TypeConstructor("Markdown"))],
        check_markdown,
        eval_markdown,
    ),
    shorthand(
        "hmd",
        "Hmd",
        &[P::Type(TypeConstructor("Hmd"))],
        check_hmd,
        eval_hmd,
    ),
    shorthand(
        "json",
        "Json",
        &[P::Type(TypeConstructor("Json"))],
        check_json,
        eval_json,
    ),
    shorthand(
        "yaml",
        "Yaml",
        &[P::Type(TypeConstructor("Yaml"))],
        check_yaml,
        eval_yaml,
    ),
    shorthand(
        "csv",
        "Csv",
        &[P::Type(TypeConstructor("Csv"))],
        check_csv,
        eval_csv,
    ),
    shorthand(
        "d2",
        "D2",
        &[P::Type(TypeConstructor("D2"))],
        check_d2,
        eval_d2,
    ),
    shorthand(
        "mermaid",
        "Mermaid",
        &[P::Type(TypeConstructor("Mermaid"))],
        check_mermaid,
        eval_mermaid,
    ),
];

/// One check and one evaluation per shorthand, each with its format fixed.
macro_rules! shorthands {
    ($($check:ident, $eval:ident => $format:literal),* $(,)?) => {
        $(fn $check(
            _: &dyn super::CheckCx,
            call: &super::spec::CheckedCall<'_>,
            span: Range<usize>,
        ) -> Result<(), Diagnostic> {
            writable(call.input, $format).map_err(|message| Diagnostic::typing(span, message))
        }

        fn $eval(
            cx: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value, Diagnostic> {
            written(cx, &input, TypeConstructor($format), 0, None, span)
        })*
    };
}

shorthands! {
    check_text, eval_text => "Text",
    check_markdown, eval_markdown => "Markdown",
    check_hmd, eval_hmd => "Hmd",
    check_json, eval_json => "Json",
    check_yaml, eval_yaml => "Yaml",
    check_csv, eval_csv => "Csv",
    check_d2, eval_d2 => "D2",
    check_mermaid, eval_mermaid => "Mermaid",
}

fn check_format(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let format = call
        .bindings
        .get("F")
        .map_or("", |format| format.constructor.0);
    writable(call.input, format).map_err(|message| Diagnostic::typing(span, message))
}

/// A value with an optional wrapper removed, since presence does not change
/// how a value is written.
fn unwrapped(value: &Value) -> &Value {
    match value {
        Value::Present(held) => unwrapped(held),
        other => other,
    }
}

/// `name := format(Format)`: fixes the format and nothing else.
const fn shorthand(
    name: &'static str,
    format: &'static str,
    piece: &'static [TypePattern],
    check: super::Check,
    eval: super::Eval,
) -> Operation {
    Operation {
        name,
        spec: OperationSpec {
            parameters: &[],
            input: P::Any,
            arguments: &[],
            output: O::Finite {
                finite: P::Type(TypeConstructor(format)),
                open: P::Apply(crate::types::collections::STREAM, piece),
            },
        },
        summary: "Shorthand for format with its format fixed.",
        purity: Purity::Pure,
        check: Some(check),
        eval,
    }
}

fn eval_format(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let format = arguments
        .iter()
        .find(|argument| argument.name.as_deref().is_none_or(|name| name == "as"))
        .map(|argument| argument.value.ty.constructor)
        .ok_or_else(|| Diagnostic::typing(span.clone(), "`format` needs a format type"))?;
    let limit = match named(arguments, "limit") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Int(n) => usize::try_from(n).unwrap_or(0),
            _ => 0,
        },
        None => 0,
    };
    let renderer = match named(arguments, "renderer") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Str(name) => Some(name.to_string()),
            _ => None,
        },
        None => None,
    };
    written(cx, &input, format, limit, renderer.as_deref(), span)
}

fn written(
    cx: &mut dyn EvalCx,
    input: &Value,
    format: TypeConstructor,
    limit: usize,
    renderer: Option<&str>,
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Value::Traversal(traversal) = input
        && traversal.forced().is_none()
    {
        // A stream that may not end is written piece by piece, as it is
        // consumed; a finite one is produced and written whole.
        if !traversal.finite {
            return Ok(Value::Traversal(Arc::new(Traversal::new(
                TypeRef::from(format),
                false,
                Source::Format {
                    input: Arc::clone(traversal),
                    format,
                    span,
                },
            ))));
        }
        let mut values = Vec::new();
        cx.traverse(traversal, &mut |_, value| {
            values.push(value);
            Ok(true)
        })?;
        let whole = Value::list(values, traversal.element.clone());
        return written(cx, &whole, format, limit, renderer, span);
    }
    let text = render(input, format, limit, renderer, &cx.vault().formats)
        .map_err(|message| Diagnostic::runtime(span, message))?;
    Ok(Value::Formatted(Arc::new(Formatted { text, format })))
}
