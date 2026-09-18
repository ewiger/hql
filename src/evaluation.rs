//! Evaluation: pure over the values it is given, with the vault as its source.

use crate::ast::{Arg, Expr, Kind, Program, Stmt};
use crate::checker::{self, StepRef, step_of};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::extensions::{EvalCx, Resolution, Step};
use crate::graph::Edge;
use crate::search;
use crate::types::TypeRef;
use crate::types::Value;
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
                Stmt::Bind {
                    name,
                    value,
                    annotation,
                } => {
                    let mut evaluated = self.expression(value)?;
                    if let Some(annotation) = annotation {
                        evaluated =
                            evaluated.viewed_as(&crate::declarations::annotation(annotation)?);
                    }
                    self.scope.insert(name.clone(), evaluated);
                    Value::Unit
                }
                Stmt::Import { name, span } => {
                    self.resolution.import(name, span)?;
                    Value::Unit
                }
                // Checking established what the declaration says; evaluating
                // it has nothing left to do.
                Stmt::Type(_) => Value::Unit,
                Stmt::Expr(expression) => self.expression(expression)?,
            };
        }
        Ok(last)
    }

    fn expression(&mut self, expression: &Expr) -> Result<Value, Diagnostic> {
        let span = expression.span.clone();
        match &expression.kind {
            Kind::List(expressions) | Kind::Set(expressions) => {
                let mut values = Vec::new();
                let mut element = TypeRef::NEVER;
                for expression in expressions {
                    let value = self.expression(expression)?;
                    element = element.common(&value.type_of()).ok_or_else(|| {
                        Diagnostic::runtime(span.clone(), "incompatible collection elements")
                    })?;
                    values.push(value);
                }
                Ok(if matches!(expression.kind, Kind::Set(_)) {
                    Value::set(values, element)
                } else {
                    Value::list(values, element)
                })
            }
            Kind::Construct {
                annotation,
                arguments,
            } => {
                let expected = crate::declarations::annotation(annotation)?;
                crate::constructors::evaluate(
                    self,
                    expected.constructor,
                    arguments,
                    Some(&expected),
                    span,
                )
            }
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
                TypeRef::CARD,
            )),
            Kind::DocRef(name) => match self.vault.resolve(name) {
                Some(card) => Ok(Value::Present(Rc::new(Value::Card(card)))),
                None => {
                    // Permitted: a forward link to a document nobody has
                    // written yet. Said out loud, because absence is easy to
                    // mistake for a match.
                    self.warnings.push(Warning::new(
                        span,
                        format!("`[[{name}]]` does not resolve in this vault"),
                    ));
                    Ok(Value::Absent(TypeRef::CARD))
                }
            },
            Kind::Ident(name) => self
                .scope
                .get(name)
                .cloned()
                .ok_or_else(|| Diagnostic::name(span, format!("`{name}` is not bound"))),
            Kind::Add(..) => self.addition(expression),
            Kind::Compare {
                left,
                right,
                negated,
            } => {
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                let equal = if matches!(left, Value::Data(_)) || matches!(right, Value::Data(_)) {
                    as_data(&left) == as_data(&right)
                } else {
                    left == right
                };
                Ok(Value::Bool(equal != *negated))
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
            Kind::Call { callee, arguments } => {
                let Kind::Ident(name) = &callee.kind else {
                    return Err(Diagnostic::typing(span, "expected a named function"));
                };
                if let Some(constructor) = crate::constructors::named(name) {
                    return crate::constructors::evaluate(self, constructor, arguments, None, span);
                }
                if name == "compare" {
                    let [left, right] = arguments.as_slice() else {
                        return Err(Diagnostic::runtime(span, "compare expects two values"));
                    };
                    let left = self.expression(&left.value)?.order_key().ok_or_else(|| {
                        Diagnostic::runtime(span.clone(), "left value is not Orderable")
                    })?;
                    let right = self.expression(&right.value)?.order_key().ok_or_else(|| {
                        Diagnostic::runtime(span.clone(), "right value is not Orderable")
                    })?;
                    return Ok(Value::Ordering(left.compare(&right)));
                }
                let Some((input, arguments)) = arguments.split_first() else {
                    return Err(Diagnostic::runtime(span, "a function needs an input"));
                };
                let input = self.expression(&input.value)?;
                let step = self.resolution.step(name, &span)?;
                self.step(step, input, arguments, span)
            }
            Kind::Pipe(input, step) => {
                let value = self.expression(input)?;
                let reference = step_of(step)?;
                let resolved = self.resolve(&reference, &step.span)?;
                self.step(resolved, value, reference.arguments(), step.span.clone())
            }
        }
    }

    /// Fold a left-associated chain, retaining each partial sum's overflow span.
    fn addition(&mut self, mut expression: &Expr) -> Result<Value, Diagnostic> {
        let mut operands = Vec::new();
        while let Kind::Add(left, right) = &expression.kind {
            operands.push((expression, left.as_ref(), right.as_ref()));
            expression = left;
        }
        let mut sum = self.expression(expression)?;
        for (addition, left, right) in operands.into_iter().rev() {
            let overflow = || Diagnostic::Overflow {
                span: addition.span.clone(),
            };
            sum = match (sum, self.expression(right)?) {
                (Value::Int(a), Value::Int(b)) => {
                    a.checked_add(b).map(Value::Int).ok_or_else(overflow)?
                }
                (Value::Float(a), Value::Float(b)) => {
                    let sum = a + b;
                    if !sum.is_finite() {
                        return Err(overflow());
                    }
                    Value::Float(sum)
                }
                (a, b) => {
                    return Err(Diagnostic::typing(
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
                    ));
                }
            };
        }
        Ok(sum)
    }

    /// The name a link endpoint denotes, whether or not it resolves.
    fn endpoint(&mut self, expression: &Expr) -> Result<String, Diagnostic> {
        if let Kind::DocRef(name) = &expression.kind {
            return Ok(name.clone());
        }
        match self.expression(expression)? {
            Value::Present(value) => match value.as_ref() {
                Value::Card(card) => Ok(card.name().to_owned()),
                Value::Doc(doc) => Ok(doc.name.clone()),
                _ => Err(Diagnostic::typing(
                    expression.span.clone(),
                    "a link endpoint must be a document",
                )),
            },
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
            Value::Present(value) => self
                .field(value, field, span)
                .map(|value| Value::Present(Rc::new(value))),
            Value::Map(map) if field == "keys" => Ok(map.keys()),
            // Absence propagates rather than becoming a failure, because the
            // reference that produced it was permitted.
            Value::Absent(element) => Ok(Value::Absent(
                checker::field_type(element, field).unwrap_or(TypeRef::DATA),
            )),
            Value::Data(data) => Ok(data
                .get(field)
                .map_or(Value::Absent(TypeRef::DATA), |found| {
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
                "hits" => Ok(Value::list(
                    ranking
                        .hits
                        .iter()
                        .map(|hit| Value::Hit(Rc::new(hit.clone())))
                        .collect(),
                    TypeRef::hit(TypeRef::CARD),
                )),
                "query" => Ok(text(&ranking.retrieval.query)),
                "retrieval" => Ok(Value::Data(Rc::new(retrieval_data(&ranking.retrieval)))),
                _ => Err(missing()),
            },
            Value::Graph(graph) => match field {
                "nodes" => Ok(Value::list(
                    graph.nodes.iter().map(|name| text(name)).collect(),
                    TypeRef::STR,
                )),
                "edges" => Ok(Value::list(
                    graph
                        .induced()
                        .into_iter()
                        .map(|edge| Value::Edge(Rc::new(edge.clone())))
                        .collect(),
                    TypeRef::EDGE,
                )),
                _ => Err(missing()),
            },
            _ => Err(missing()),
        }
    }

    /// Evaluate a lambda argument once per element.
    fn apply(&mut self, argument: &Arg, values: Vec<Value>) -> Result<Value, Diagnostic> {
        let Kind::Lambda { parameters, body } = &argument.value.kind else {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                "expected a lambda",
            ));
        };
        if parameters.len() != values.len() {
            return Err(Diagnostic::runtime(
                argument.value.span.clone(),
                "wrong lambda arity",
            ));
        }
        let previous = parameters
            .iter()
            .zip(values)
            .map(|(name, value)| (name, self.scope.insert(name.clone(), value)))
            .collect::<Vec<_>>();
        let result = self.expression(body);
        for (name, value) in previous {
            match value {
                Some(value) => {
                    self.scope.insert(name.clone(), value);
                }
                None => {
                    self.scope.remove(name);
                }
            }
        }
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
            Value::Present(value) => value.as_ref().clone(),
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
        self.apply(argument, vec![element])
    }

    fn apply_comparator(
        &mut self,
        argument: &Arg,
        left: Value,
        right: Value,
    ) -> Result<Value, Diagnostic> {
        self.apply(argument, vec![left, right])
    }

    fn warn(&mut self, warning: Warning) {
        self.warnings.push(warning);
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
    data.insert_path("revision", Data::Str(retrieval.revision.clone()));
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
        Value::Present(value) => as_data(value),
        Value::List(values, _) | Value::Set(values, _) => {
            Data::List(values.iter().map(as_data).collect())
        }
        other => Data::Str(other.to_string()),
    }
}
