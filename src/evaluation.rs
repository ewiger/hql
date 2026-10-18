//! Evaluation: pure over the values it is given, with the vault as its source.

use crate::ast::{Arith, Logic};
use crate::execution::{Arg, Expr, Field, Kind, Program, Stmt};

use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::extensions::{EvalCx, Step};
use crate::graph::Edge;
use crate::reporting::{Report, TraceStep};
use crate::search;
use crate::types::TypeRef;
use crate::types::Value;
use crate::vault::Vault;
use crate::warnings::Warning;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Evaluate a checked program, logging anything worth saying along the way.
///
/// The log comes back whether or not evaluation failed, so what a program
/// captured before its first failure is not lost with the value.
pub(crate) fn evaluate(
    program: &Program,
    vault: &Vault,
) -> (Result<Value, Diagnostic>, Vec<Report>) {
    let mut scope = HashMap::new();
    if vault.is_present() {
        // Built once here, not on each mention of the name.
        let cards = vault
            .cards
            .iter()
            .map(|card| Value::Card(Arc::clone(card)))
            .collect();
        scope.insert(
            crate::checker::CARDS.to_owned(),
            Value::set(cards, TypeRef::CARD),
        );
    }
    let mut evaluator = Evaluator {
        vault,
        scope,
        log: Vec::new(),
        frames: Vec::new(),
    };
    let value = evaluator.program(program);
    (value, evaluator.log)
}

struct Evaluator<'a> {
    vault: &'a Vault,
    scope: HashMap<String, Value>,
    /// Entries in the order evaluation produced them, which on this
    /// sequential evaluator is the order of their traces.
    log: Vec<Report>,
    /// The steps being evaluated, outermost first.
    frames: Vec<Frame>,
}

/// A step under evaluation, and the pipeline steps that fed it.
struct Frame {
    step: &'static str,
    upstream: Vec<&'static str>,
    element: Option<String>,
}

impl Evaluator<'_> {
    fn program(&mut self, program: &Program) -> Result<Value, Diagnostic> {
        let mut last = Value::Unit;
        for statement in &program.statements {
            last = match statement {
                Stmt::Bind { name, value, view } => {
                    let evaluated = self.expression(value)?.viewed_as(view);
                    self.scope.insert(name.clone(), evaluated);
                    Value::Unit
                }
                Stmt::Unit => Value::Unit,
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
                let element = expression.ty.element().unwrap_or(TypeRef::NEVER);
                for expression in expressions {
                    values.push(self.expression(expression)?);
                }
                Ok(if matches!(expression.kind, Kind::Set(_)) {
                    Value::set(values, element)
                } else {
                    Value::list(values, element)
                })
            }
            Kind::Construct(arguments) => {
                crate::constructors::evaluate(self, arguments, &expression.ty, span)
            }
            Kind::Int(value) => Ok(Value::Int(*value)),
            Kind::Float(value) => Ok(Value::Float(*value)),
            Kind::Bool(value) => Ok(Value::Bool(*value)),
            Kind::Str(text) => Ok(Value::Str(Arc::from(text.as_str()))),
            Kind::Data(entries) => {
                let mut data = Data::map();
                for (key, value) in entries {
                    data.insert_path(key, as_data(&self.expression(value)?));
                }
                Ok(Value::Data(Arc::new(data)))
            }
            Kind::DocRef(name) => match self.vault.resolve(name) {
                Some(card) => Ok(Value::Present(Arc::new(Value::Card(card)))),
                None => {
                    // Permitted: a forward link to a document nobody has
                    // written yet. Said out loud, because absence is easy to
                    // mistake for a match.
                    self.warn(Warning::new(
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
            Kind::Arith { first, rest } => self.arithmetic(first, rest),
            Kind::Relate {
                left,
                right,
                relation,
            } => {
                let key = |value: Value| {
                    value.order_key().ok_or_else(|| {
                        Diagnostic::runtime(span.clone(), "checked ordering value has no key")
                    })
                };
                let left = key(self.expression(left)?)?;
                let right = key(self.expression(right)?)?;
                Ok(Value::Bool(relation.holds(left.compare(&right))))
            }
            Kind::Logic { first, rest } => {
                let mut value = self.condition(first)?;
                for (connective, operand) in rest {
                    // Short-circuit: the right operand is evaluated only when
                    // it can still change the answer.
                    let decided = match connective {
                        Logic::And => !value,
                        Logic::Or => value,
                    };
                    if !decided {
                        value = self.condition(operand)?;
                    }
                }
                Ok(Value::Bool(value))
            }
            Kind::Not(operand) => Ok(Value::Bool(!self.condition(operand)?)),
            Kind::Compare {
                left,
                right,
                negated,
            } => {
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                // Equality looks through `Option`: absence equals absence and
                // nothing present.
                let equal = match (present(left), present(right)) {
                    (None, None) => true,
                    (Some(left), Some(right))
                        if matches!(left, Value::Data(_)) || matches!(right, Value::Data(_)) =>
                    {
                        as_data(&left) == as_data(&right)
                    }
                    (Some(left), Some(right)) => left == right,
                    _ => false,
                };
                Ok(Value::Bool(equal != *negated))
            }
            Kind::Field(receiver, field) => {
                let value = self.expression(receiver)?;
                Self::field(&value, field, &expression.ty, &span)
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
                Ok(Value::Edge(Arc::new(Edge::link(&from, &to, annotation))))
            }
            Kind::Lambda { .. } => Err(Diagnostic::typing(
                span,
                "a lambda may only be written as an argument",
            )),
            Kind::Order(left, right) => {
                let left = self.expression(left)?.order_key().ok_or_else(|| {
                    Diagnostic::runtime(span.clone(), "checked ordering value has no key")
                })?;
                let right = self.expression(right)?.order_key().ok_or_else(|| {
                    Diagnostic::runtime(span.clone(), "checked ordering value has no key")
                })?;
                Ok(Value::Ordering(left.compare(&right)))
            }
            Kind::Step {
                step,
                input,
                arguments,
                at,
            } => {
                let value = self.expression(input)?;
                self.frames.push(Frame {
                    step: step.name,
                    upstream: upstream(input),
                    element: None,
                });
                // A step's own reports point at the step, not at the whole
                // pipeline that feeds it.
                let result = self.step(step, value, arguments, at.clone());
                self.frames.pop();
                result
            }
            Kind::Type => Err(Diagnostic::runtime(span, "a type argument is not a value")),
        }
    }

    /// Fold a left-associated chain, retaining each partial result's span.
    fn arithmetic(
        &mut self,
        first: &Expr,
        rest: &[(Arith, Expr, Range<usize>)],
    ) -> Result<Value, Diagnostic> {
        let mut result = self.expression(first)?;
        for (operator, right, span) in rest {
            let overflow = || Diagnostic::Overflow { span: span.clone() };
            result = match (result, self.expression(right)?) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 && matches!(operator, Arith::Div | Arith::Rem) {
                        return Err(Diagnostic::runtime(span.clone(), "division by zero"));
                    }
                    // Division truncates toward zero, and `%` keeps the sign
                    // of the dividend, so `a == (a / b) * b + a % b`.
                    match operator {
                        Arith::Add => a.checked_add(b),
                        Arith::Sub => a.checked_sub(b),
                        Arith::Mul => a.checked_mul(b),
                        Arith::Div => a.checked_div(b),
                        Arith::Rem => a.checked_rem(b),
                    }
                    .map(Value::Int)
                    .ok_or_else(overflow)?
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 && matches!(operator, Arith::Div | Arith::Rem) {
                        return Err(Diagnostic::runtime(span.clone(), "division by zero"));
                    }
                    let result = match operator {
                        Arith::Add => a + b,
                        Arith::Sub => a - b,
                        Arith::Mul => a * b,
                        Arith::Div => a / b,
                        Arith::Rem => a % b,
                    };
                    if !result.is_finite() {
                        return Err(overflow());
                    }
                    Value::Float(result)
                }
                _ => {
                    return Err(Diagnostic::runtime(
                        span.clone(),
                        format!("invalid checked {}", operator.noun()),
                    ));
                }
            };
        }
        Ok(result)
    }

    /// Evaluate a checked connective operand.
    fn condition(&mut self, operand: &Expr) -> Result<bool, Diagnostic> {
        self.expression(operand)?.truth().ok_or_else(|| {
            Diagnostic::runtime(operand.span.clone(), "checked condition is not a Bool")
        })
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

    fn field(
        value: &Value,
        field: &Field,
        result: &TypeRef,
        span: &Range<usize>,
    ) -> Result<Value, Diagnostic> {
        let missing = || {
            Diagnostic::name(
                span.clone(),
                format!("{} has no field `{field:?}`", value.type_of()),
            )
        };
        match value {
            Value::Present(value) => {
                Self::field(value, field, result.present(), span).map(|found| match found {
                    Value::Present(_) | Value::Absent(_) => found,
                    other => Value::Present(Arc::new(other)),
                })
            }
            Value::Map(map) if matches!(field, Field::Keys) => Ok(map.keys()),
            // Absence propagates rather than becoming a failure, because the
            // reference that produced it was permitted.
            Value::Absent(_) => Ok(Value::Absent(result.present().clone())),
            Value::Data(data) => {
                let Field::Key(key) = field else {
                    return Err(missing());
                };
                data_key(data, key, span)
            }
            Value::Card(card) => match field {
                Field::Name => Ok(text(card.name())),
                Field::Title => Ok(text(card.document.title())),
                Field::Path => Ok(text(&card.document.path.display().to_string())),
                Field::Format => Ok(text(card.document.format.name())),
                Field::Body => Ok(text(&card.document.body)),
                Field::Header => Ok(Value::Data(Arc::new(card.document.header.clone()))),
                Field::Metadata => Ok(Value::Data(Arc::new(card.metadata.clone()))),
                Field::Kind => Ok(text(match card.kind {
                    CardKind::Concept => "ConceptCard",
                    CardKind::Relation => "RelationCard",
                })),
                _ => Err(missing()),
            },
            Value::Doc(doc) => match field {
                Field::Name => Ok(text(&doc.name)),
                Field::Title => Ok(text(doc.title())),
                Field::Path => Ok(text(&doc.path.display().to_string())),
                Field::Format => Ok(text(doc.format.name())),
                Field::Body => Ok(text(&doc.body)),
                Field::Header => Ok(Value::Data(Arc::new(doc.header.clone()))),
                _ => Err(missing()),
            },
            Value::Edge(edge) => match field {
                Field::Source => Ok(text(&edge.source)),
                Field::Target => Ok(text(&edge.target)),
                Field::Data => Ok(Value::Data(Arc::new(edge.data.clone()))),
                _ => Err(missing()),
            },
            Value::Hit(hit) => match field {
                Field::Card => Ok(Value::Card(Arc::clone(&hit.card))),
                Field::Score => Ok(Value::Float(hit.score)),
                Field::Query => Ok(text(&hit.provenance.query)),
                Field::Retrieval => Ok(Value::Data(Arc::new(retrieval_data(&hit.provenance)))),
                _ => Err(missing()),
            },
            Value::Ranking(ranking) => match field {
                Field::Hits => Ok(Value::list(
                    ranking
                        .hits
                        .iter()
                        .map(|hit| Value::Hit(Arc::new(hit.clone())))
                        .collect(),
                    TypeRef::hit(TypeRef::CARD),
                )),
                Field::Query => Ok(text(&ranking.retrieval.query)),
                Field::Retrieval => Ok(Value::Data(Arc::new(retrieval_data(&ranking.retrieval)))),
                _ => Err(missing()),
            },
            Value::Graph(graph) => match field {
                Field::Nodes => Ok(Value::list(
                    graph.nodes.iter().map(|name| text(name)).collect(),
                    TypeRef::STR,
                )),
                Field::Edges => Ok(Value::list(
                    graph
                        .induced()
                        .into_iter()
                        .map(|edge| Value::Edge(Arc::new(edge.clone())))
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
        // The element a step is applying its function to joins the trace,
        // so an entry logged inside the lambda says which element it was.
        let element = match values.as_slice() {
            [value] => Some(element_name(value)),
            _ => None,
        };
        let outer = self
            .frames
            .last_mut()
            .map(|frame| std::mem::replace(&mut frame.element, element));
        let previous = parameters
            .iter()
            .zip(values)
            .map(|(name, value)| (name, self.scope.insert(name.clone(), value)))
            .collect::<Vec<_>>();
        let result = self.expression(body);
        if let (Some(frame), Some(outer)) = (self.frames.last_mut(), outer) {
            frame.element = outer;
        }
        for (name, value) in previous.into_iter().rev() {
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

    /// Dispatch a pipeline step to the extension that provides it.
    fn step(
        &mut self,
        step: &'static Step,
        input: Value,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Value, Diagnostic> {
        if step.keeps_input() {
            return (step.eval)(self, input, arguments, span);
        }
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
        self.log.push(Report::of_warning(&warning));
    }

    fn log(&mut self, entry: Report) {
        self.log.push(entry);
    }

    fn trace(&self) -> Vec<TraceStep> {
        let mut trace = Vec::new();
        for frame in &self.frames {
            for (index, step) in frame.upstream.iter().enumerate() {
                trace.push(TraceStep {
                    step: (*step).to_owned(),
                    position: index + 1,
                    element: None,
                });
            }
            trace.push(TraceStep {
                step: frame.step.to_owned(),
                position: frame.upstream.len() + 1,
                element: frame.element.clone(),
            });
        }
        trace
    }
}

/// The steps a pipeline ran before the one whose input this is, first first.
fn upstream(mut input: &Expr) -> Vec<&'static str> {
    let mut steps = Vec::new();
    while let Kind::Step {
        step, input: inner, ..
    } = &input.kind
    {
        steps.push(step.name);
        input = inner;
    }
    steps.reverse();
    steps
}

/// How a trace names an element: a document by its name, anything else as
/// it prints, shortened.
fn element_name(value: &Value) -> String {
    match value {
        Value::Card(card) => card.name().to_owned(),
        Value::Doc(doc) => doc.name.clone(),
        Value::Hit(hit) => hit.card.name().to_owned(),
        Value::Present(inner) => element_name(inner),
        other => crate::reporting::shorten(&other.to_string()),
    }
}

/// Read one key of an open tree, as `Option<Data>`.
///
/// Nothing promised the key, so reading it is optional by type and a missing
/// one is absence. A tree that is not a map at all is a different matter: the
/// program viewed `Data` as its map alternative and the value is not one, so
/// that view fails as a type error rather than passing for absence. An empty
/// entry holds nothing, so nothing can be read out of it.
pub(crate) fn data_key(data: &Data, key: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
    match data {
        Data::Map(entries) => Ok(entries
            .get(key)
            .map_or(Value::Absent(TypeRef::DATA), |found| {
                Value::Present(Arc::new(Value::Data(Arc::new(found.clone()))))
            })),
        Data::Empty => Ok(Value::Absent(TypeRef::DATA)),
        other => Err(Diagnostic::typing(
            span.clone(),
            format!(
                "`{key}` needs a map here, not Data of shape {}",
                other.shape()
            ),
        )),
    }
}

/// The value under any number of `Option` layers, or `None` when it is absent.
fn present(value: Value) -> Option<Value> {
    match value {
        Value::Absent(_) => None,
        Value::Present(inner) => present(Arc::unwrap_or_clone(inner)),
        other => Some(other),
    }
}

fn text(value: &str) -> Value {
    Value::Str(Arc::from(value))
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
