//! Evaluation: pure over the values it is given, with the vault as its source.

use crate::ast::{Arith, Logic};
use crate::execution::{Arg, BlockStmt, Expr, Field, Kind, Program, Stmt};
use crate::traversal::{Source, Traversal};

use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::document::Kind as CardKind;
use crate::extensions::{EvalCx, Operation};
use crate::graph::Edge;
use crate::reporting::{Report, TraceStep};
use crate::search;
use crate::types::TypeRef;
use crate::types::{Closure, Value};
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
    evaluate_with(program, vault, Vec::new())
}

/// Evaluate a program with further names bound: the values of a document's
/// other cells.
pub(crate) fn evaluate_with(
    program: &Program,
    vault: &Vault,
    bound: Vec<(String, Value)>,
) -> (Result<Value, Diagnostic>, Vec<Report>) {
    let mut scope = base(vault);
    // The library's bindings over `vault`, each its own program.
    for (name, library) in crate::checker::library(vault) {
        let (value, log) = evaluate_in(&library, vault, base(vault));
        match value {
            Ok(value) => {
                scope.insert(name, value);
            }
            Err(diagnostic) => return (Err(diagnostic), log),
        }
    }
    scope.extend(bound);
    evaluate_in(program, vault, scope)
}

/// Drive a stream a program returned, handing each value to `sink` until the
/// values run out or the sink stops: what a host does to write a stream as it
/// arrives.
pub(crate) fn traverse(
    traversal: &Traversal,
    vault: &Vault,
    sink: &mut dyn FnMut(Value) -> bool,
) -> Result<(), Diagnostic> {
    let mut evaluator = Evaluator {
        vault,
        scope: base(vault),
        log: Vec::new(),
        frames: Vec::new(),
    };
    evaluator
        .run_traversal(traversal, &mut |_, value| Ok(sink(value)))
        .map(drop)
}

/// What a host seeds before the library: `vault`, when there is one.
fn base(vault: &Vault) -> HashMap<String, Value> {
    let mut scope = HashMap::new();
    if vault.is_present() {
        scope.insert(crate::checker::VAULT.to_owned(), Value::Vault);
    }
    scope
}

fn evaluate_in(
    program: &Program,
    vault: &Vault,
    scope: HashMap<String, Value>,
) -> (Result<Value, Diagnostic>, Vec<Report>) {
    let mut evaluator = Evaluator {
        vault,
        scope,
        log: Vec::new(),
        frames: Vec::new(),
    };
    // A finite traversal is the values it produces, so the result holds them;
    // a stream that may not end is left for the host to bound.
    let value = match evaluator.program(program) {
        Ok(Value::Traversal(traversal)) if traversal.finite => evaluator.force(&traversal, &(0..0)),
        other => other,
    };
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
                Stmt::Fn {
                    name,
                    parameters,
                    body,
                    ty,
                } => {
                    let function = Value::Function(Arc::new(Closure {
                        name: Some(name.clone()),
                        parameters: parameters.clone(),
                        body: Arc::clone(body),
                        captured: self.scope.clone(),
                        ty: ty.clone(),
                    }));
                    self.scope.insert(name.clone(), function);
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
            Kind::List(expressions) | Kind::Set(expressions) => {
                let mut values = Vec::new();
                let element = expression.ty.element().unwrap_or(TypeRef::NEVER);
                for expression in expressions {
                    let value = self.expression(expression)?;
                    values.push(self.settled(value, &span)?);
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
                    let value = self.expression(value)?;
                    data.insert_path(key, as_data(&self.settled(value, &span)?));
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
            Kind::Ident(name) => match self.scope.get(name) {
                Some(value) => Ok(value.clone()),
                None if name == "none" => Ok(Value::Absent(TypeRef::NEVER)),
                None => Err(Diagnostic::name(span, format!("`{name}` is not bound"))),
            },
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
                self.field(&value, field, &expression.ty, &span)
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
            Kind::Combine {
                combiner,
                left,
                right,
            } => {
                let left = self.expression(left)?;
                let left = self.settled(left, &span)?;
                let right = self.expression(right)?;
                let right = self.settled(right, &span)?;
                combine(combiner, left, right, &expression.ty, &span)
            }
            Kind::Generate {
                statements,
                element,
                finite,
            } => Ok(Value::Traversal(Arc::new(Traversal::new(
                element.clone(),
                *finite,
                Source::Block {
                    statements: Arc::clone(statements),
                    scope: self.scope.clone(),
                },
            )))),
            Kind::Lambda {
                parameters,
                body,
                function,
            } => Ok(Value::Function(Arc::new(Closure {
                name: None,
                parameters: parameters.clone(),
                body: Arc::clone(body),
                captured: self.scope.clone(),
                ty: function.clone().unwrap_or_else(|| expression.ty.clone()),
            }))),
            Kind::Apply {
                callee,
                arguments,
                pack,
            } => {
                let callee = self.expression(callee)?;
                let mut values = arguments
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some((fixed, element)) = pack {
                    let trailing = values.split_off((*fixed).min(values.len()));
                    values.push(Value::list(trailing, element.clone()));
                }
                self.call(&callee, values, &span)
            }
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
                let value = as_checked(self.expression(input)?, &input.ty);
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
            Kind::Some(value) => Ok(Value::Present(Arc::new(self.expression(value)?))),
            Kind::Tee {
                input,
                name,
                branch,
            } => {
                let value = self.expression(input)?;
                let previous = self
                    .scope
                    .insert(crate::ast::TEE_INPUT.to_owned(), value.clone());
                let result = self.expression(branch);
                match previous {
                    Some(previous) => self
                        .scope
                        .insert(crate::ast::TEE_INPUT.to_owned(), previous),
                    None => self.scope.remove(crate::ast::TEE_INPUT),
                };
                self.scope.insert(name.clone(), result?);
                Ok(value)
            }
            Kind::Given {
                value,
                condition,
                otherwise,
            } => {
                let holds = self.expression(condition)?.truth().ok_or_else(|| {
                    Diagnostic::runtime(condition.span.clone(), "a condition is a Bool")
                })?;
                match (holds, otherwise) {
                    (true, None) => Ok(Value::Present(Arc::new(self.expression(value)?))),
                    (true, Some(_)) => self.expression(value),
                    (false, None) => Ok(Value::Absent(value.ty.clone())),
                    (false, Some(other)) => self.expression(other),
                }
            }
            Kind::Match { subject, arms } => {
                let subject = self.expression(subject)?;
                for arm in arms {
                    let mut bindings = Vec::new();
                    if !fits(&arm.pattern, &subject, &mut bindings) {
                        continue;
                    }
                    let previous = bindings
                        .into_iter()
                        .map(|(name, value)| {
                            let held = self.scope.insert(name.clone(), value);
                            (name, held)
                        })
                        .collect::<Vec<_>>();
                    let outcome = (|| {
                        if let Some(guard) = &arm.guard {
                            let holds = self.expression(guard)?.truth().ok_or_else(|| {
                                Diagnostic::runtime(guard.span.clone(), "a guard is a Bool")
                            })?;
                            if !holds {
                                return Ok(None);
                            }
                        }
                        self.expression(&arm.body).map(Some)
                    })();
                    for (name, held) in previous.into_iter().rev() {
                        match held {
                            Some(held) => {
                                self.scope.insert(name, held);
                            }
                            None => {
                                self.scope.remove(&name);
                            }
                        }
                    }
                    if let Some(result) = outcome? {
                        return Ok(result);
                    }
                }
                Err(Diagnostic::runtime(span, "no match arm fits this value"))
            }
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
        &self,
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
                self.field(value, field, result.present(), span)
                    .map(|found| match found {
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
                Field::Value => Ok(hit.value.clone()),
                Field::Score => Ok(Value::Float(hit.score)),
                Field::Query => Ok(text(&hit.provenance.query)),
                Field::Retrieval => Ok(Value::Data(Arc::new(retrieval_data(&hit.provenance)))),
                _ => Err(missing()),
            },
            Value::Vault => match field {
                Field::Cards => Ok(Value::Traversal(Arc::new(Traversal::new(
                    TypeRef::CARD,
                    true,
                    Source::Cards,
                )))),
                _ => Err(missing()),
            },
            Value::Ranking(ranking) => match field {
                Field::Hits => Ok(Value::list(
                    ranking
                        .hits
                        .iter()
                        .map(|hit| Value::Hit(Arc::new(hit.clone())))
                        .collect(),
                    TypeRef::hit(ranking.element.clone()),
                )),
                Field::Query => Ok(text(&ranking.retrieval.query)),
                Field::Retrieval => Ok(Value::Data(Arc::new(retrieval_data(&ranking.retrieval)))),
                _ => Err(missing()),
            },
            Value::Group(group) => match field {
                Field::GroupKey => Ok(group.key.clone()),
                Field::Items => Ok(Value::list(group.items.clone(), group.item_type.clone())),
                _ => Err(missing()),
            },
            Value::Graph(graph) => match field {
                // A node is a card of the vault; a graph built over a vault
                // never names one it does not hold.
                Field::Nodes => Ok(Value::set(
                    graph
                        .nodes
                        .iter()
                        .filter_map(|name| self.vault.resolve(name))
                        .map(Value::Card)
                        .collect(),
                    TypeRef::CARD,
                )),
                Field::Edges => Ok(Value::set(
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

    /// Call a function value: its body runs in the scope it captured, with
    /// its parameters bound and, for a declared function, its own name.
    fn call(
        &mut self,
        callee: &Value,
        values: Vec<Value>,
        span: &Range<usize>,
    ) -> Result<Value, Diagnostic> {
        let Value::Function(function) = callee else {
            return Err(Diagnostic::runtime(
                span.clone(),
                "only a function can be called",
            ));
        };
        if function.parameters.len() != values.len() {
            return Err(Diagnostic::runtime(
                span.clone(),
                "wrong number of arguments",
            ));
        }
        let mut scope = function.captured.clone();
        if let Some(name) = &function.name {
            scope.insert(name.clone(), callee.clone());
        }
        scope.extend(function.parameters.iter().cloned().zip(values));
        let outer = std::mem::replace(&mut self.scope, scope);
        let result = self.expression(&function.body);
        self.scope = outer;
        result
    }

    /// Drive a traversal, handing each value to `sink` until the values run
    /// out or the sink stops. Answers whether the sink would take more.
    fn run_traversal(
        &mut self,
        traversal: &Traversal,
        sink: &mut dyn FnMut(&mut Self, Value) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        if let Some(values) = traversal.forced() {
            for value in values {
                if !sink(self, value.clone())? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        match &traversal.source {
            Source::Block { statements, scope } => {
                let outer = std::mem::replace(&mut self.scope, scope.clone());
                let more = self.run_block(statements, sink);
                self.scope = outer;
                more
            }
            Source::Cards => {
                // A sorted walk: path components compare one by one, so a
                // directory's entries follow it in name order, depth first.
                let mut walk: Vec<_> = self.vault.cards.iter().collect();
                walk.sort_by(|left, right| left.document.path.cmp(&right.document.path));
                for card in walk {
                    if !sink(self, Value::Card(Arc::clone(card)))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Source::Map { input, by, span } => {
                self.run_traversal(input, &mut |evaluator, value| {
                    let mapped = evaluator.call(by, vec![value], span)?;
                    sink(evaluator, mapped)
                })
            }
            Source::Filter { input, by, span } => self.run_traversal(
                input,
                &mut |evaluator, value| match evaluator.call(by, vec![value.clone()], span)? {
                    Value::Bool(true) => sink(evaluator, value),
                    _ => Ok(true),
                },
            ),
            Source::Take { input, n } => {
                if *n == 0 {
                    return Ok(true);
                }
                let (mut seen, mut stopped) = (0, false);
                self.run_traversal(input, &mut |evaluator, value| {
                    seen += 1;
                    if !sink(evaluator, value)? {
                        stopped = true;
                        return Ok(false);
                    }
                    Ok(seen < *n)
                })?;
                Ok(!stopped)
            }
            Source::Collect { input, n } => {
                let element = input.element.clone();
                let mut batch = Vec::with_capacity(*n);
                let mut stopped = false;
                self.run_traversal(input, &mut |evaluator, value| {
                    batch.push(value);
                    if batch.len() < *n {
                        return Ok(true);
                    }
                    let full = Value::list(std::mem::take(&mut batch), element.clone());
                    let more = sink(evaluator, full)?;
                    stopped = !more;
                    Ok(more)
                })?;
                if stopped {
                    return Ok(false);
                }
                // The last batch holds what remained, and an empty input makes
                // no batch at all.
                if batch.is_empty() {
                    Ok(true)
                } else {
                    sink(self, Value::list(batch, element))
                }
            }
            Source::Emit { input } => self.run_traversal(input, &mut |evaluator, batch| {
                let members = match &batch {
                    Value::Traversal(inner) => {
                        let mut members = Vec::new();
                        evaluator.run_traversal(inner, &mut |_, member| {
                            members.push(member);
                            Ok(true)
                        })?;
                        members
                    }
                    other => other.elements().unwrap_or_default(),
                };
                for member in members {
                    if !sink(evaluator, member)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }),
            Source::Format {
                input,
                format,
                span,
            } => {
                let mut first = true;
                self.run_traversal(input, &mut |evaluator, value| {
                    let text = crate::extensions::format::piece(&value, *format, first)
                        .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
                    first = false;
                    let piece = Value::Formatted(Arc::new(crate::types::Formatted {
                        text,
                        format: *format,
                    }));
                    sink(evaluator, piece)
                })
            }
            // Kept values are always forced, and answered above.
            Source::Kept => Ok(true),
            Source::Drop { input, n } => {
                let mut skipped = 0;
                self.run_traversal(input, &mut |evaluator, value| {
                    if skipped < *n {
                        skipped += 1;
                        return Ok(true);
                    }
                    sink(evaluator, value)
                })
            }
        }
    }

    /// Run an iterating function's statements. A binding lasts to the end of
    /// its block.
    fn run_block(
        &mut self,
        statements: &[BlockStmt],
        sink: &mut dyn FnMut(&mut Self, Value) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        let outer = self.scope.clone();
        let more = self.run_statements(statements, sink);
        self.scope = outer;
        more
    }

    fn run_statements(
        &mut self,
        statements: &[BlockStmt],
        sink: &mut dyn FnMut(&mut Self, Value) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        for statement in statements {
            match statement {
                BlockStmt::Bind { name, value } => {
                    let value = self.expression(value)?;
                    self.scope.insert(name.clone(), value);
                }
                BlockStmt::Yield(value) => {
                    let value = self.expression(value)?;
                    if !sink(self, value)? {
                        return Ok(false);
                    }
                }
                BlockStmt::For {
                    name,
                    iterable,
                    body,
                } => {
                    let over = match self.expression(iterable)? {
                        Value::Present(held) => held.as_ref().clone(),
                        Value::Absent(_) => continue,
                        other => other,
                    };
                    let more = if let Value::Traversal(traversal) = &over {
                        self.run_traversal(traversal, &mut |evaluator, value| {
                            evaluator.scope.insert(name.clone(), value);
                            evaluator.run_block(body, sink)
                        })?
                    } else {
                        let values = over.elements().ok_or_else(|| {
                            Diagnostic::runtime(
                                iterable.span.clone(),
                                "`for` traverses a collection or a stream",
                            )
                        })?;
                        let mut more = true;
                        for value in values {
                            self.scope.insert(name.clone(), value);
                            if !self.run_block(body, sink)? {
                                more = false;
                                break;
                            }
                        }
                        more
                    };
                    if !more {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    /// A value placed inside another holds its values, so a finite traversal
    /// is forced there; a stream stays lazy.
    fn settled(&mut self, value: Value, span: &Range<usize>) -> Result<Value, Diagnostic> {
        match value {
            Value::Traversal(traversal) if traversal.finite => self.force(&traversal, span),
            other => Ok(other),
        }
    }

    /// The whole of a finite traversal, produced once and kept.
    fn force(
        &mut self,
        traversal: &Arc<Traversal>,
        span: &Range<usize>,
    ) -> Result<Value, Diagnostic> {
        if traversal.forced().is_none() {
            if !traversal.finite {
                return Err(Diagnostic::runtime(
                    span.clone(),
                    format!(
                        "{} may not end, so it cannot be used whole; bound it with `take(n)`",
                        traversal.type_of()
                    ),
                ));
            }
            let mut values = Vec::new();
            self.run_traversal(traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            traversal.keep(values);
        }
        Ok(Value::Traversal(Arc::clone(traversal)))
    }

    /// Evaluate a lambda argument once per element.
    fn apply(&mut self, argument: &Arg, values: Vec<Value>) -> Result<Value, Diagnostic> {
        let Kind::Lambda {
            parameters, body, ..
        } = &argument.value.kind
        else {
            // A function value where a lambda could be written.
            let function = self.expression(&argument.value)?;
            return self.call(&function, values, &argument.value.span);
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
        step: &'static Operation,
        input: Value,
        arguments: &[Arg],
        span: Range<usize>,
    ) -> Result<Value, Diagnostic> {
        if step.keeps_input() {
            return (step.eval)(self, input, arguments, span);
        }
        // `filter` over one optional value: absent stays absent, and a present
        // value is kept or made absent.
        if step.name == "filter" && matches!(&input, Value::Absent(_) | Value::Present(_)) {
            let held = match &input {
                Value::Present(held) => held.as_ref().clone(),
                _ => return Ok(input),
            };
            if held.elements().is_none() && !matches!(held, Value::Traversal(_)) {
                let by = crate::extensions::named_or_first(arguments, "by").ok_or_else(|| {
                    Diagnostic::runtime(span.clone(), "`filter` needs a predicate")
                })?;
                let kept = matches!(self.apply(by, vec![held.clone()])?, Value::Bool(true));
                return Ok(if kept {
                    let narrowed = crate::extensions::collections::narrowed_to(by)
                        .and_then(|wanted| held.interpreted(&wanted))
                        .unwrap_or(held);
                    Value::Present(Arc::new(narrowed))
                } else {
                    Value::Absent(held.type_of())
                });
            }
        }
        // Absence is the empty collection here, so every step sees a
        // collection and none of them has to know about it.
        let input = match input {
            Value::Present(value) => value.as_ref().clone(),
            Value::Absent(element) => Value::set(Vec::new(), element),
            // A step that needs its whole input gets a traversal's values;
            // the steps that consume one value at a time keep it lazy.
            Value::Traversal(traversal)
                if !crate::extensions::collections::LAZY.contains(&step.name) =>
            {
                self.force(&traversal, &span)?
            }
            other => other,
        };
        (step.eval)(self, input, arguments, span)
    }
}

impl EvalCx for Evaluator<'_> {
    fn vault(&self) -> &Vault {
        self.vault
    }

    fn traverse(
        &mut self,
        traversal: &Traversal,
        sink: &mut dyn FnMut(&mut dyn EvalCx, Value) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        self.run_traversal(traversal, &mut |evaluator, value| sink(evaluator, value))
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

/// Two values combined by a combiner the checker accepted for them.
pub(crate) fn combine(
    combiner: &str,
    left: Value,
    right: Value,
    ty: &TypeRef,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let failed = || Diagnostic::runtime(span.clone(), format!("`{combiner}` cannot combine these"));
    match combiner {
        "min" | "max" => {
            let order = left
                .order_key()
                .zip(right.order_key())
                .map(|(l, r)| l.compare(&r));
            let keep_left = match order {
                Some(order) if combiner == "min" => order != std::cmp::Ordering::Greater,
                Some(order) => order != std::cmp::Ordering::Less,
                None => true,
            };
            Ok(if keep_left { left } else { right })
        }
        "union" => {
            let mut values = left.elements().ok_or_else(failed)?;
            values.extend(right.elements().ok_or_else(failed)?);
            Ok(Value::set(values, ty.element().unwrap_or(TypeRef::NEVER)))
        }
        "concat" => {
            let mut values = left.elements().ok_or_else(failed)?;
            values.extend(right.elements().ok_or_else(failed)?);
            Ok(Value::list(values, ty.element().unwrap_or(TypeRef::NEVER)))
        }
        "merge" => merged(left, right, span),
        _ => Err(failed()),
    }
}

/// Every key of either map; a key in both takes the right value, at the
/// left key's position.
fn merged(left: Value, right: Value, span: &Range<usize>) -> Result<Value, Diagnostic> {
    match (&left, &right) {
        (Value::Map(l), Value::Map(r)) => {
            let mut entries: Vec<(Value, Value)> = l.entries().to_vec();
            for (key, value) in r.entries() {
                match entries.iter_mut().find(|(held, _)| held == key) {
                    Some(entry) => entry.1 = value.clone(),
                    None => entries.push((key.clone(), value.clone())),
                }
            }
            let ty = l.type_of();
            let kind = crate::types::MapKind::of(ty.constructor)
                .ok_or_else(|| Diagnostic::runtime(span.clone(), "`merge` combines two maps"))?;
            let (keys, values) = entries.into_iter().unzip();
            let map = crate::types::MapValue::new(
                kind,
                keys,
                values,
                ty.args.first().cloned().unwrap_or(TypeRef::DATA),
                ty.args.get(1).cloned().unwrap_or(TypeRef::DATA),
            )
            .map_err(|error| Diagnostic::runtime(span.clone(), error.to_string()))?;
            Ok(Value::Map(Arc::new(map)))
        }
        (Value::Data(l), Value::Data(r)) => match (l.as_ref(), r.as_ref()) {
            (Data::Map(l), Data::Map(r)) => {
                let mut entries = l.clone();
                entries.extend(r.iter().map(|(key, value)| (key.clone(), value.clone())));
                Ok(Value::Data(Arc::new(Data::Map(entries))))
            }
            (l, r) => {
                let other = if matches!(l, Data::Map(_)) { r } else { l };
                Err(Diagnostic::runtime(
                    span.clone(),
                    format!(
                        "`merge` combines two maps, not Data of shape {}",
                        other.shape()
                    ),
                ))
            }
        },
        _ => Err(Diagnostic::runtime(
            span.clone(),
            "`merge` combines two maps",
        )),
    }
}

/// A materialized list where checking established a sequence carries the
/// sequence type, so a step reading the value's type, `debug` among them,
/// sees the type the program has.
fn as_checked(value: Value, checked: &TypeRef) -> Value {
    match value {
        Value::List(values, element) if checked.constructor == crate::types::collections::SEQ => {
            Value::Traversal(Arc::new(Traversal::kept(element, values.as_ref().clone())))
        }
        other => other,
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
        Value::Hit(hit) => hit.name(),
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

/// Whether a value fits a pattern, collecting the names it binds.
fn fits(pattern: &crate::ast::Pattern, value: &Value, bindings: &mut Vec<(String, Value)>) -> bool {
    use crate::ast::Pattern;
    match (pattern, value) {
        (Pattern::Wildcard, _) => true,
        (Pattern::Bind(name), _) => {
            bindings.push((name.clone(), value.clone()));
            true
        }
        (Pattern::Some(inner), Value::Present(held)) => fits(inner, held, bindings),
        (Pattern::Type(name, inner), _) => crate::types::named(name, &[])
            .and_then(|wanted| value.interpreted(&wanted))
            .is_some_and(|interpreted| fits(inner, &interpreted, bindings)),
        (Pattern::None, Value::Absent(_)) => true,
        (Pattern::Int(expected), Value::Int(actual)) => expected == actual,
        (Pattern::Float(expected), Value::Float(actual)) => expected == actual,
        (Pattern::Str(expected), Value::Str(actual)) => expected.as_str() == actual.as_ref(),
        (Pattern::Bool(expected), Value::Bool(actual)) => expected == actual,
        // A tree leaf compares by its value, as equality does.
        (
            Pattern::Str(_) | Pattern::Int(_) | Pattern::Float(_) | Pattern::Bool(_),
            Value::Data(_),
        ) => {
            let literal = match pattern {
                Pattern::Str(text) => Value::Str(text.as_str().into()),
                Pattern::Int(n) => Value::Int(*n),
                Pattern::Float(x) => Value::Float(*x),
                Pattern::Bool(b) => Value::Bool(*b),
                _ => return false,
            };
            as_data(&literal) == as_data(value)
        }
        _ => false,
    }
}
