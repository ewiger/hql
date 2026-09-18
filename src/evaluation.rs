//! Evaluation: pure over the values it is given, with the vault as its source.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::checker::{self, stage_of};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::graph::{Edge, Graph, Presence};
use crate::search::{self, Ranking};
use crate::types::Type;
use crate::values::{Presentation, Value};
use crate::vault::Vault;
use crate::warnings::Warning;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ops::Range;
use std::rc::Rc;

/// Evaluate a checked program, collecting anything worth saying along the way.
///
/// # Errors
///
/// Returns the first evaluation failure.
pub(crate) fn evaluate(
    program: &Program,
    vault: &Vault,
) -> Result<(Value, Vec<Warning>), Diagnostic> {
    let mut evaluator = Evaluator {
        vault,
        scope: HashMap::new(),
        warnings: Vec::new(),
    };
    let value = evaluator.program(program)?;
    Ok((value, evaluator.warnings))
}

struct Evaluator<'a> {
    vault: &'a Vault,
    scope: HashMap<String, Value>,
    warnings: Vec<Warning>,
}

impl Evaluator<'_> {
    fn program(&mut self, program: &Program) -> Result<Value, Diagnostic> {
        let mut last = Value::Unit;
        for statement in &program.statements {
            last = match statement {
                Stmt::Bind { name, value, .. } => {
                    let evaluated = self.expression(value)?;
                    self.scope.insert(name.clone(), evaluated);
                    Value::Unit
                }
                Stmt::Expr(expression) => self.expression(expression)?,
            };
        }
        Ok(last)
    }

    fn expression(&mut self, expression: &Expr) -> Result<Value, Diagnostic> {
        let span = expression.span.clone();
        match &expression.kind {
            Kind::Int(value) => Ok(Value::Int(*value)),
            Kind::Float(value) => Ok(Value::Float(*value)),
            Kind::Bool(value) => Ok(Value::Bool(*value)),
            Kind::Str(text) => Ok(Value::Str(Rc::from(text.as_str()))),
            Kind::Data(entries) => {
                let mut data = Data::map();
                for (key, value) in entries {
                    data.insert_path(key, as_data(&self.expression(value)?));
                }
                Ok(Value::Data(Rc::new(data)))
            }
            Kind::Cards => Ok(Value::set(
                self.vault
                    .cards
                    .iter()
                    .map(|card| Value::Card(Rc::clone(card)))
                    .collect(),
                Type::Card,
            )),
            Kind::DocRef(name) => match self.vault.resolve(name) {
                Some(card) => Ok(Value::Card(card)),
                None => {
                    // Permitted: a forward link to a document nobody has
                    // written yet. Said out loud, because absence is easy to
                    // mistake for a match.
                    self.warnings.push(Warning::new(
                        span,
                        format!("`[[{name}]]` does not resolve in this vault"),
                    ));
                    Ok(Value::Absent(Type::Card))
                }
            },
            Kind::Ident(name) => self
                .scope
                .get(name)
                .cloned()
                .ok_or_else(|| Diagnostic::name(span, format!("`{name}` is not bound"))),
            Kind::Add(left, right) => {
                let overflow = || Diagnostic::Overflow { span: span.clone() };
                match (self.expression(left)?, self.expression(right)?) {
                    (Value::Int(a), Value::Int(b)) => {
                        a.checked_add(b).map(Value::Int).ok_or_else(overflow)
                    }
                    // Values stay finite, so a saturating sum is reported.
                    (Value::Float(a), Value::Float(b)) => {
                        let sum = a + b;
                        sum.is_finite()
                            .then_some(Value::Float(sum))
                            .ok_or_else(overflow)
                    }
                    (a, b) => Err(Diagnostic::typing(
                        if matches!(a, Value::Int(_) | Value::Float(_)) {
                            right.span.clone()
                        } else {
                            left.span.clone()
                        },
                        format!(
                            "addition requires two Int or two Float operands, not {} and {}",
                            a.type_of(),
                            b.type_of()
                        ),
                    )),
                }
            }
            Kind::Compare {
                left,
                right,
                negated,
            } => {
                let left = as_data(&self.expression(left)?);
                let right = as_data(&self.expression(right)?);
                Ok(Value::Bool((left == right) != *negated))
            }
            Kind::Field(receiver, field) => {
                let value = self.expression(receiver)?;
                self.field(&value, field, &span)
            }
            Kind::Link {
                source,
                target,
                data,
            } => {
                let from = self.endpoint(source)?;
                let to = self.endpoint(target)?;
                let annotation = match data {
                    Some(literal) => as_data(&self.expression(literal)?),
                    None => Data::map(),
                };
                Ok(Value::Edge(Rc::new(Edge::link(&from, &to, annotation))))
            }
            Kind::Lambda { .. } => Err(Diagnostic::typing(
                span,
                "a lambda may only be written as an argument",
            )),
            Kind::Call { .. } => Err(Diagnostic::typing(
                span,
                "a stage needs an input: write `… | stage(…)`",
            )),
            Kind::Pipe(input, stage) => {
                let value = self.expression(input)?;
                let (name, arguments) = stage_of(stage)?;
                self.stage(name, value, arguments, stage.span.clone())
            }
        }
    }

    /// The name a link endpoint denotes, whether or not it resolves.
    fn endpoint(&mut self, expression: &Expr) -> Result<String, Diagnostic> {
        if let Kind::DocRef(name) = &expression.kind {
            return Ok(name.clone());
        }
        match self.expression(expression)? {
            Value::Str(text) => Ok(text.to_string()),
            Value::Card(card) => Ok(card.name().to_owned()),
            Value::Doc(doc) => Ok(doc.name.clone()),
            other => Err(Diagnostic::typing(
                expression.span.clone(),
                format!(
                    "a link endpoint is a document reference, not {}",
                    other.type_of()
                ),
            )),
        }
    }

    fn field(&self, value: &Value, field: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
        let missing = || {
            Diagnostic::name(
                span.clone(),
                format!("{} has no field `{field}`", value.type_of()),
            )
        };
        match value {
            // Absence propagates rather than becoming a failure, because the
            // reference that produced it was permitted.
            Value::Absent(element) => Ok(Value::Absent(
                checker::field_type(element, field).unwrap_or(Type::Data),
            )),
            Value::Data(data) => Ok(data.get(field).map_or(Value::Absent(Type::Data), |found| {
                Value::Data(Rc::new(found.clone()))
            })),
            Value::Card(card) => match field {
                "name" => Ok(text(card.name())),
                "title" => Ok(text(card.document.title())),
                "path" => Ok(text(&card.document.path.display().to_string())),
                "format" => Ok(text(card.document.format.name())),
                "body" => Ok(text(&card.document.body)),
                "header" => Ok(Value::Data(Rc::new(card.document.header.clone()))),
                "metadata" => Ok(Value::Data(Rc::new(card.metadata.clone()))),
                "kind" => Ok(text(match card.kind {
                    CardKind::Concept => "ConceptCard",
                    CardKind::Relation => "RelationCard",
                })),
                _ => Err(missing()),
            },
            Value::Doc(doc) => match field {
                "name" => Ok(text(&doc.name)),
                "title" => Ok(text(doc.title())),
                "path" => Ok(text(&doc.path.display().to_string())),
                "format" => Ok(text(doc.format.name())),
                "body" => Ok(text(&doc.body)),
                "header" => Ok(Value::Data(Rc::new(doc.header.clone()))),
                _ => Err(missing()),
            },
            Value::Edge(edge) => match field {
                "source" => Ok(text(&edge.source)),
                "target" => Ok(text(&edge.target)),
                "data" => Ok(Value::Data(Rc::new(edge.data.clone()))),
                _ => Err(missing()),
            },
            Value::Hit(hit) => match field {
                "card" => Ok(Value::Card(Rc::clone(&hit.card))),
                "score" => Ok(Value::Float(hit.score)),
                "query" => Ok(text(&hit.provenance.query)),
                "retrieval" => Ok(Value::Data(Rc::new(retrieval_data(&hit.provenance)))),
                _ => Err(missing()),
            },
            Value::Ranking(ranking) => match field {
                "hits" => Ok(Value::seq(
                    ranking
                        .hits
                        .iter()
                        .map(|hit| Value::Hit(Rc::new(hit.clone())))
                        .collect(),
                    Type::Hit(Box::new(Type::Card)),
                )),
                "query" => Ok(text(&ranking.retrieval.query)),
                "retrieval" => Ok(Value::Data(Rc::new(retrieval_data(&ranking.retrieval)))),
                _ => Err(missing()),
            },
            Value::Graph(graph) => match field {
                "nodes" => Ok(Value::seq(
                    graph.nodes.iter().map(|name| text(name)).collect(),
                    Type::Str,
                )),
                "edges" => Ok(Value::seq(
                    graph
                        .induced()
                        .into_iter()
                        .map(|edge| Value::Edge(Rc::new(edge.clone())))
                        .collect(),
                    Type::Edge,
                )),
                _ => Err(missing()),
            },
            _ => Err(missing()),
        }
    }

    /// Evaluate a lambda argument once per element.
    fn apply(&mut self, argument: &Arg, element: Value) -> Result<Value, Diagnostic> {
        let Kind::Lambda { parameter, body } = &argument.value.kind else {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "expected a function",
            ));
        };
        let shadowed = self.scope.insert(parameter.clone(), element);
        let result = self.expression(body);
        match shadowed {
            Some(previous) => self.scope.insert(parameter.clone(), previous),
            None => self.scope.remove(parameter),
        };
        result
    }

    fn stage(
        &mut self,
        name: &str,
        input: Value,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Value, Diagnostic> {
        // Absence is the empty collection here, so every stage below sees a
        // collection and none of them has to know about it.
        let input = match input {
            Value::Absent(element) => Value::set(Vec::new(), element),
            other => other,
        };
        let wrong = |what: &str| Diagnostic::typing(span.clone(), format!("`{name}` needs {what}"));

        match name {
            "count" => Ok(Value::Int(
                input
                    .elements()
                    .ok_or_else(|| wrong("a collection"))?
                    .len()
                    .try_into()
                    .unwrap_or(i64::MAX),
            )),
            "sort" => {
                let mut elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                match named_or_first(arguments, "by") {
                    Some(argument) => {
                        let mut keyed = Vec::new();
                        for element in elements {
                            let key = self.apply(argument, element.clone())?;
                            keyed.push((key.order_key(), element));
                        }
                        keyed.sort_by(|left, right| compare(&left.0, &right.0));
                        elements = keyed.into_iter().map(|(_, element)| element).collect();
                    }
                    None => elements
                        .sort_by(|left, right| compare(&left.order_key(), &right.order_key())),
                }
                Ok(Value::seq(elements, element_type(&input)))
            }
            "take" => {
                let argument =
                    named_or_first(arguments, "n").ok_or_else(|| wrong("a count: `take(5)`"))?;
                let Value::Int(count) = self.expression(&argument.value)? else {
                    return Err(wrong("an Int count"));
                };
                let count = usize::try_from(count).map_err(|_| {
                    Diagnostic::runtime(argument.value.span.clone(), "a count cannot be negative")
                })?;
                if let Value::Ranking(ranking) = &input {
                    // Already ordered by score, and that order is the point.
                    return Ok(Value::Ranking(Rc::new(Ranking {
                        hits: ranking.hits.iter().take(count).cloned().collect(),
                        retrieval: Rc::clone(&ranking.retrieval),
                    })));
                }
                let mut elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                // A set has no order, so the prefix comes from the elements'
                // own. A sequence already has one, and imposing another here
                // would discard the `sort` that produced it.
                if matches!(input, Value::Set(..)) {
                    elements.sort_by(|left, right| compare(&left.order_key(), &right.order_key()));
                }
                elements.truncate(count);
                Ok(Value::seq(elements, element_type(&input)))
            }
            "filter" => {
                let argument =
                    named_or_first(arguments, "by").ok_or_else(|| wrong("a predicate"))?;
                let elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                let mut kept = Vec::new();
                for element in elements {
                    let verdict = self.apply(argument, element.clone())?;
                    if verdict.truth().ok_or_else(|| {
                        Diagnostic::typing(
                            argument.value.span.clone(),
                            "a filter predicate returns Bool",
                        )
                    })? {
                        kept.push(element);
                    }
                }
                Ok(rebuild(&input, kept))
            }
            "map" => {
                let argument =
                    named_or_first(arguments, "by").ok_or_else(|| wrong("a function"))?;
                let elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                let mut mapped = Vec::new();
                for element in elements {
                    mapped.push(self.apply(argument, element)?);
                }
                let element = mapped
                    .first()
                    .map_or(Type::Data, crate::values::Value::type_of);
                Ok(Value::seq(mapped, element))
            }
            "typed" => {
                let argument =
                    named_or_first(arguments, "as").ok_or_else(|| wrong("a type name"))?;
                let Kind::Ident(type_name) = &argument.value.kind else {
                    return Err(wrong("a type name"));
                };
                let wanted = crate::types::named(type_name, &[]).ok_or_else(|| {
                    Diagnostic::name(
                        argument.value.span.clone(),
                        format!("unknown type `{type_name}`"),
                    )
                })?;
                let elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                // The kind was checked when the vault loaded; an authored
                // entry alone never put a card in this result.
                let kept: Vec<Value> = elements
                    .into_iter()
                    .filter(|element| element.type_of().is(&wanted))
                    .collect();
                Ok(Value::set(kept, wanted))
            }
            "uplinks" | "downlinks" => {
                let subjects = input.elements().unwrap_or_else(|| vec![input.clone()]);
                let mut edges = Vec::new();
                for subject in subjects {
                    let card = subject.as_card().ok_or_else(|| wrong("a card or cards"))?;
                    let found = if name == "uplinks" {
                        self.vault.uplinks(card.name())
                    } else {
                        self.vault.downlinks(card.name())
                    };
                    edges.extend(
                        found
                            .into_iter()
                            .map(|edge| Value::Edge(Rc::new(edge.clone()))),
                    );
                }
                Ok(Value::seq(edges, Type::Edge))
            }
            "semantic" => {
                let argument =
                    named_or_first(arguments, "query").ok_or_else(|| wrong("a query"))?;
                let Value::Str(query) = self.expression(&argument.value)? else {
                    return Err(wrong("a text query"));
                };
                let elements = input.elements().ok_or_else(|| wrong("a collection"))?;
                let cards: Vec<Rc<crate::document::Card>> = elements
                    .iter()
                    .filter_map(crate::values::Value::as_card)
                    .collect();
                if cards.len() != elements.len() {
                    return Err(wrong("a collection of cards"));
                }
                Ok(Value::Ranking(Rc::new(search::rank(
                    &cards,
                    &query,
                    &self.vault.index_name(),
                ))))
            }
            "expand" => {
                let depth = match named_or_first(arguments, "depth") {
                    Some(argument) => match self.expression(&argument.value)? {
                        Value::Int(value) => usize::try_from(value).map_err(|_| {
                            Diagnostic::runtime(
                                argument.value.span.clone(),
                                "a depth cannot be negative",
                            )
                        })?,
                        _ => return Err(wrong("an Int depth")),
                    },
                    None => 1,
                };
                let direction = match named(arguments, "direction") {
                    Some(argument) => match self.expression(&argument.value)? {
                        Value::Str(text) => text.to_string(),
                        _ => return Err(wrong("a text direction")),
                    },
                    None => "both".to_owned(),
                };
                if !matches!(direction.as_str(), "uplinks" | "downlinks" | "both") {
                    return Err(Diagnostic::runtime(
                        span,
                        format!(
                            "`direction` is `uplinks`, `downlinks` or `both`, not `{direction}`"
                        ),
                    ));
                }
                let seeded = self.project(&input, &span)?;
                Ok(Value::Graph(Rc::new(
                    self.expand(seeded, depth, &direction),
                )))
            }
            "graph" => {
                if let Value::Graph(_) = input {
                    // Projecting a graph yields the graph: the traversal that
                    // built it already produced the structure.
                    return Ok(input);
                }
                let projected = self.project(&input, &span)?;
                Ok(Value::Graph(Rc::new(projected)))
            }
            "table" => Ok(present("table", input.to_table())),
            "json" => Ok(present(
                "json",
                serde_json::to_string_pretty(&input.to_json())
                    .unwrap_or_else(|_| "null".to_owned()),
            )),
            "text" => Ok(present("text", input.to_string())),
            other => Err(Diagnostic::name(span, format!("`{other}` is not a stage"))),
        }
    }

    /// Project a collection of cards into a graph, recording why each node is
    /// present. A relation card contributes its edge and does not become a
    /// node; its endpoints arrive as the nodes the edge needs.
    fn project(&self, input: &Value, span: &Range<usize>) -> Result<Graph, Diagnostic> {
        if let Value::Graph(graph) = input {
            return Ok(graph.as_ref().clone());
        }
        let elements = input.elements().unwrap_or_else(|| vec![input.clone()]);
        let mut presence: BTreeMap<String, Presence> = BTreeMap::new();
        let mut edges: Vec<Edge> = Vec::new();
        for element in elements {
            let card = element.as_card().ok_or_else(|| {
                Diagnostic::typing(span.clone(), "a graph is projected from cards")
            })?;
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
                    self.vault
                        .edges
                        .iter()
                        .filter(|edge| edge.source == source && edge.target == target)
                        .cloned(),
                );
                continue;
            }
            presence.insert(card.name().to_owned(), how);
        }
        edges.extend(self.vault.edges.iter().cloned());
        Ok(Graph::new(presence, edges))
    }

    /// Breadth-first traversal outwards from the nodes already present.
    fn expand(&self, seeded: Graph, depth: usize, direction: &str) -> Graph {
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
                    self.vault
                        .uplinks(&node)
                        .into_iter()
                        .map(|edge| edge.target.clone()),
                );
            }
            if direction != "uplinks" {
                neighbours.extend(
                    self.vault
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
}

fn text(value: &str) -> Value {
    Value::Str(Rc::from(value))
}

fn present(presenter: &str, rendered: String) -> Value {
    Value::Presentation(Rc::new(Presentation {
        presenter: presenter.to_owned(),
        text: rendered,
    }))
}

fn retrieval_data(retrieval: &search::Retrieval) -> Data {
    let mut data = Data::map();
    data.insert_path("query", Data::Str(retrieval.query.clone()));
    data.insert_path("index", Data::Str(retrieval.index.clone()));
    data.insert_path("model", Data::Str(retrieval.model.clone()));
    data.insert_path("metric", Data::Str(retrieval.metric.clone()));
    data.insert_path("approximate", Data::Bool(retrieval.approximate));
    data
}

fn as_data(value: &Value) -> Data {
    match value {
        Value::Int(number) => Data::Int(*number),
        Value::Float(number) => Data::Float(*number),
        Value::Bool(truth) => Data::Bool(*truth),
        Value::Str(text) => Data::Str(text.to_string()),
        Value::Data(data) => data.as_ref().clone(),
        Value::Absent(_) | Value::Unit => Data::Empty,
        other => Data::Str(other.to_string()),
    }
}

fn element_type(input: &Value) -> Type {
    input.type_of().element().unwrap_or(Type::Data)
}

/// Keep a filtered collection in the shape it came in.
fn rebuild(input: &Value, kept: Vec<Value>) -> Value {
    match input {
        Value::Set(_, element) => Value::set(kept, element.clone()),
        Value::Ranking(_) => Value::seq(kept, Type::Hit(Box::new(Type::Card))),
        _ => Value::seq(kept, element_type(input)),
    }
}

fn compare(
    left: &Option<crate::values::Key>,
    right: &Option<crate::values::Key>,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.compare(right),
        _ => std::cmp::Ordering::Equal,
    }
}

fn named<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some(name))
}

fn named_or_first<'a>(arguments: &'a [Arg], name: &str) -> Option<&'a Arg> {
    named(arguments, name).or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
}
