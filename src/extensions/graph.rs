//! The `graph` extension: traversal and projection.
//!
//! Drawing a graph is not the core's business. The layering is core → graph →
//! knowledge, and these steps are the graph layer's half of it.

use super::collections::wrong;
use super::spec::{ArgumentSpec, Output as O, StepSpec, TypePattern as P};
use super::{EvalCx, Purity, Step, named, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::execution::Arg as TypedArg;
use crate::graph::{Edge, Graph, Presence};
use crate::types::TypeRef;
use crate::types::Value;
use crate::vault::Vault;
use std::collections::{BTreeMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `graph` extension provides.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "uplinks",
        spec: StepSpec {
            parameters: &[],
            input: P::OneOf(&[
                P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ]),
                P::Collection(&P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ])),
            ]),
            arguments: &[],
            output: O::Type(P::Apply(
                crate::types::collections::LIST,
                &[P::Type(crate::types::TypeConstructor("Edge"))],
            )),
        },
        summary: "The edges leaving a card.",
        purity: Purity::Pure,
        check: None,
        eval: eval_uplinks,
    },
    Step {
        name: "downlinks",
        spec: StepSpec {
            parameters: &[],
            input: P::OneOf(&[
                P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ]),
                P::Collection(&P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ])),
            ]),
            arguments: &[],
            output: O::Type(P::Apply(
                crate::types::collections::LIST,
                &[P::Type(crate::types::TypeConstructor("Edge"))],
            )),
        },
        summary: "The edges entering a card, which needs a vault.",
        purity: Purity::Pure,
        check: Some(check_downlinks),
        eval: eval_downlinks,
    },
    Step {
        name: "expand",
        spec: StepSpec {
            parameters: &[],
            input: P::OneOf(&[
                P::Type(crate::types::TypeConstructor("Graph")),
                P::Type(crate::types::TypeConstructor("Doc")),
                P::Collection(&P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ])),
            ]),
            arguments: &[
                ArgumentSpec {
                    required: false,
                    ..ArgumentSpec::value("depth", P::Type(crate::types::TypeConstructor("Int")))
                },
                ArgumentSpec {
                    required: false,
                    positional: false,
                    ..ArgumentSpec::value(
                        "direction",
                        P::Type(crate::types::TypeConstructor("String")),
                    )
                },
            ],
            output: O::Type(P::Type(crate::types::TypeConstructor("Graph"))),
        },
        summary: "Traverse outwards, keeping why each node is present.",
        purity: Purity::Pure,
        check: Some(check_expand),
        eval: eval_expand,
    },
    Step {
        name: "graph",
        spec: StepSpec {
            parameters: &[],
            input: P::OneOf(&[
                P::Type(crate::types::TypeConstructor("Graph")),
                P::Type(crate::types::TypeConstructor("Doc")),
                P::Collection(&P::OneOf(&[
                    P::Type(crate::types::TypeConstructor("Doc")),
                    P::Apply(
                        crate::types::TypeConstructor("Hit"),
                        &[P::Type(crate::types::TypeConstructor("Card"))],
                    ),
                ])),
            ]),
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Graph"))),
        },
        summary: "Project a collection of cards into a graph.",
        purity: Purity::Pure,
        check: None,
        eval: eval_graph,
    },
];

fn eval_uplinks(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    eval_links(cx, &input, "uplinks", &span)
}

fn eval_downlinks(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    eval_links(cx, &input, "downlinks", &span)
}

fn eval_links(
    cx: &mut dyn EvalCx,
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let subjects = input.elements().unwrap_or_else(|| vec![input.clone()]);
    let mut edges = Vec::new();
    for subject in subjects {
        let card = subject
            .as_card()
            .ok_or_else(|| wrong(name, "a card or cards", span))?;
        let found = if name == "uplinks" {
            cx.vault().uplinks(card.name())
        } else {
            cx.vault().downlinks(card.name())
        };
        edges.extend(
            found
                .into_iter()
                .map(|edge| Value::Edge(Arc::new(edge.clone()))),
        );
    }
    Ok(Value::list(edges, TypeRef::EDGE))
}

fn eval_expand(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let depth = match named_or_first(arguments, "depth") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Int(value) => usize::try_from(value).map_err(|_| {
                Diagnostic::runtime(argument.value.span.clone(), "a depth cannot be negative")
            })?,
            _ => return Err(wrong("expand", "an Int depth", &span)),
        },
        None => 1,
    };
    let direction = match named(arguments, "direction") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Str(text) => text.to_string(),
            _ => return Err(wrong("expand", "a text direction", &span)),
        },
        None => "both".to_owned(),
    };
    if !matches!(direction.as_str(), "uplinks" | "downlinks" | "both") {
        return Err(Diagnostic::runtime(
            span,
            format!("`direction` is `uplinks`, `downlinks` or `both`, not `{direction}`"),
        ));
    }
    let seeded = project(cx.vault(), &input, &span)?;
    Ok(Value::Graph(Arc::new(expand(
        cx.vault(),
        seeded,
        depth,
        &direction,
    ))))
}

fn eval_graph(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Value::Graph(_) = input {
        // Projecting a graph yields the graph: the traversal that built it
        // already produced the structure.
        return Ok(input);
    }
    let projected = project(cx.vault(), &input, &span)?;
    Ok(Value::Graph(Arc::new(projected)))
}

/// Project a collection of cards into a graph, recording why each node is
/// present. A relation card contributes its edge and does not become a node;
/// its endpoints arrive as the nodes the edge needs.
fn project(vault: &Vault, input: &Value, span: &Range<usize>) -> Result<Graph, Diagnostic> {
    if let Value::Graph(graph) = input {
        return Ok(graph.as_ref().clone());
    }
    let elements = input.elements().unwrap_or_else(|| vec![input.clone()]);
    let mut presence: BTreeMap<String, Presence> = BTreeMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    for element in elements {
        let card = element
            .as_card()
            .ok_or_else(|| Diagnostic::typing(span.clone(), "a graph is projected from cards"))?;
        let how = match &element {
            Value::Hit(hit) => Presence::Retrieved(hit.score),
            _ => Presence::Member,
        };
        if card.kind == CardKind::Relation {
            let Some((source, target)) = card.endpoints() else {
                continue;
            };
            for end in [&source, &target] {
                presence.entry(end.clone()).or_insert(Presence::Expanded {
                    seed: card.name().to_owned(),
                    depth: 1,
                });
            }
            edges.extend(
                vault
                    .edges
                    .iter()
                    .filter(|edge| edge.source == source && edge.target == target)
                    .cloned(),
            );
            continue;
        }
        presence.insert(card.name().to_owned(), how);
    }
    edges.extend(vault.edges.iter().cloned());
    Ok(Graph::new(presence, edges))
}

/// Breadth-first traversal outwards from the nodes already present.
fn expand(vault: &Vault, seeded: Graph, depth: usize, direction: &str) -> Graph {
    let mut presence = seeded.presence;
    let mut queue: VecDeque<(String, String, usize)> = presence
        .keys()
        .map(|name| (name.clone(), name.clone(), 0))
        .collect();
    while let Some((node, seed, distance)) = queue.pop_front() {
        if distance == depth {
            continue;
        }
        let mut neighbours = Vec::new();
        if direction != "downlinks" {
            neighbours.extend(
                vault
                    .uplinks(&node)
                    .into_iter()
                    .map(|edge| edge.target.clone()),
            );
        }
        if direction != "uplinks" {
            neighbours.extend(
                vault
                    .downlinks(&node)
                    .into_iter()
                    .map(|edge| edge.source.clone()),
            );
        }
        for neighbour in neighbours {
            if presence.contains_key(&neighbour) {
                continue;
            }
            presence.insert(
                neighbour.clone(),
                Presence::Expanded {
                    seed: seed.clone(),
                    depth: distance + 1,
                },
            );
            queue.push_back((neighbour, seed.clone(), distance + 1));
        }
    }
    Graph::new(presence, seeded.edges)
}

fn check_downlinks(
    cx: &dyn super::CheckCx,
    _: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if !cx.vault().is_present() {
        return Err(Diagnostic::name(
            span,
            "`downlinks` needs a vault: a document cannot know what points at it",
        ));
    }
    Ok(())
}
fn check_expand(
    cx: &dyn super::CheckCx,
    _: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if !cx.vault().is_present() {
        return Err(Diagnostic::name(span, "`expand` needs a vault to traverse"));
    }
    Ok(())
}
