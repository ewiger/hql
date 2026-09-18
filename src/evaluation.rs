//! Evaluation: pure over the values it is given, with the vault as its source.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::checker::{self, StepRef, step_of};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::extensions::{EvalCx, Resolution, Step};
use crate::graph::Edge;
use crate::search;
use crate::types::Type;
use crate::values::Value;
use crate::vault::Vault;
use crate::warnings::Warning;
use std::collections::HashMap;
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
        resolution: Resolution::new(&vault.extensions)?,
    };
    let value = evaluator.program(program)?;
    Ok((value, evaluator.warnings))
}

struct Evaluator<'a> {
    vault: &'a Vault,
    scope: HashMap<String, Value>,
    warnings: Vec<Warning>,
    /// Which extensions this program has imported.
    resolution: Resolution,
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
                Stmt::Import { name, span } => {
                    self.resolution.import(name, span)?;
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
                "a pipeline step needs an input: write `… | step(…)`",
            )),
            Kind::Pipe(input, step) => {
                let value = self.expression(input)?;
                let reference = step_of(step)?;
                let resolved = self.resolve(&reference, &step.span)?;
                self.step(resolved, value, reference.arguments(), step.span.clone())
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

    /// Which step a written reference names, given what is imported.
    fn resolve(
        &self,
        reference: &StepRef<'_>,
        span: &Range<usize>,
    ) -> Result<&'static Step, Diagnostic> {
        match reference {
            StepRef::Bare { name, .. } => self.resolution.step(name, span),
            StepRef::Qualified {
                extension, name, ..
            } => {
                if self.scope.contains_key(*extension) {
                    return Err(Diagnostic::typing(
                        span.clone(),
                        format!(
                            "`{extension}` is bound to a value here, so `{extension}.{name}` \
                             reads a field rather than naming a step"
                        ),
                    ));
                }
                self.resolution.qualified(extension, name, span)
            }
        }
    }

    /// Dispatch a pipeline step to the extension that provides it.
    fn step(
        &mut self,
        step: &'static Step,
        input: Value,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Value, Diagnostic> {
        // Absence is the empty collection here, so every step sees a
        // collection and none of them has to know about it.
        let input = match input {
            Value::Absent(element) => Value::set(Vec::new(), element),
            other => other,
        };
        (step.eval)(self, input, arguments, span)
    }
}

impl EvalCx for Evaluator<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn evaluate(&mut self, expression: &Expr) -> Result<Value, Diagnostic> {
        self.expression(expression)
    }

    fn apply_lambda(&mut self, argument: &Arg, element: Value) -> Result<Value, Diagnostic> {
        self.apply(argument, element)
    }
}

fn text(value: &str) -> Value {
    Value::Str(Rc::from(value))
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
