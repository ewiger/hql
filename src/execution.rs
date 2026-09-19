//! Checked execution tree: resolved operations, result types, and source spans.

use crate::ast;
use crate::diagnostics::Diagnostic;
use crate::extensions::Step;
use crate::types::TypeRef;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

pub(crate) type Arg = ast::Arg<Expr>;

#[derive(Debug)]
pub(crate) struct Program {
    pub statements: Vec<Stmt>,
    pub result: TypeRef,
}

#[derive(Debug)]
pub(crate) enum Stmt {
    Bind {
        name: String,
        view: TypeRef,
        value: Expr,
    },
    Expr(Expr),
    Unit,
}

#[derive(Debug)]
pub(crate) struct Expr {
    pub kind: Kind,
    pub ty: TypeRef,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub(crate) enum Kind {
    List(Vec<Expr>),
    Set(Vec<Expr>),
    Construct(Vec<Arg>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Ident(String),
    Cards,
    DocRef(String),
    Data(Vec<(String, Expr)>),
    Add {
        first: Box<Expr>,
        rest: Vec<(Expr, Range<usize>)>,
    },
    Compare {
        left: Box<Expr>,
        right: Box<Expr>,
        negated: bool,
    },
    Order(Box<Expr>, Box<Expr>),
    Step {
        step: &'static Step,
        input: Box<Expr>,
        arguments: Vec<Arg>,
    },
    Link {
        source: Box<Expr>,
        target: Box<Expr>,
        data: Option<Box<Expr>>,
    },
    Field(Box<Expr>, Field),
    Lambda {
        parameters: Vec<String>,
        body: Box<Expr>,
    },
    Type,
}

/// A record field selected during checking; only Data keeps open string keys.
#[derive(Debug, Clone)]
pub(crate) enum Field {
    Key(String),
    Name,
    Title,
    Path,
    Format,
    Body,
    Header,
    Metadata,
    Kind,
    Source,
    Target,
    Data,
    Card,
    Score,
    Query,
    Retrieval,
    Hits,
    Nodes,
    Edges,
    Keys,
}

impl Field {
    pub fn resolve(receiver: &TypeRef, name: &str) -> Option<Self> {
        if *receiver.present() == TypeRef::DATA {
            return Some(Self::Key(name.to_owned()));
        }
        Some(match name {
            "name" => Self::Name,
            "title" => Self::Title,
            "path" => Self::Path,
            "format" => Self::Format,
            "body" => Self::Body,
            "header" => Self::Header,
            "metadata" => Self::Metadata,
            "kind" => Self::Kind,
            "source" => Self::Source,
            "target" => Self::Target,
            "data" => Self::Data,
            "card" => Self::Card,
            "score" => Self::Score,
            "query" => Self::Query,
            "retrieval" => Self::Retrieval,
            "hits" => Self::Hits,
            "nodes" => Self::Nodes,
            "edges" => Self::Edges,
            "keys" => Self::Keys,
            _ => return None,
        })
    }
}

/// Facts established by checking, consumed when syntax becomes execution IR.
#[derive(Default)]
pub(crate) struct Checked {
    pub types: HashMap<Range<usize>, TypeRef>,
    pub type_arguments: HashSet<Range<usize>>,
    pub fields: HashMap<Range<usize>, Field>,
    pub views: HashMap<Range<usize>, TypeRef>,
    pub steps: HashMap<Range<usize>, &'static Step>,
}

impl Checked {
    pub fn program(&self, program: &ast::Program, result: TypeRef) -> Result<Program, Diagnostic> {
        let statements = program
            .statements
            .iter()
            .map(|statement| {
                Ok(match statement {
                    ast::Stmt::Bind { name, value, .. } => {
                        let value = self.expression(value)?;
                        let view = self
                            .views
                            .get(&value.span)
                            .cloned()
                            .unwrap_or_else(|| value.ty.clone());
                        Stmt::Bind {
                            name: name.clone(),
                            view,
                            value,
                        }
                    }
                    ast::Stmt::Expr(value) => Stmt::Expr(self.expression(value)?),
                    ast::Stmt::Import { .. } | ast::Stmt::Type(_) => Stmt::Unit,
                })
            })
            .collect::<Result<_, Diagnostic>>()?;
        Ok(Program { statements, result })
    }

    fn arguments(&self, arguments: &[ast::Arg]) -> Result<Vec<Arg>, Diagnostic> {
        arguments
            .iter()
            .map(|argument| {
                let value = if self.type_arguments.contains(&argument.value.span) {
                    let ast::Kind::Ident(_) = &argument.value.kind else {
                        return Err(Diagnostic::typing(
                            argument.value.span.clone(),
                            "expected a type name",
                        ));
                    };
                    Expr {
                        kind: Kind::Type,
                        ty: self
                            .types
                            .get(&argument.value.span)
                            .cloned()
                            .ok_or_else(|| {
                                Diagnostic::name(
                                    argument.value.span.clone(),
                                    "unchecked type argument",
                                )
                            })?,
                        span: argument.value.span.clone(),
                    }
                } else {
                    self.expression(&argument.value)?
                };
                Ok(Arg {
                    name: argument.name.clone(),
                    value,
                })
            })
            .collect()
    }

    fn expression(&self, expression: &ast::Expr) -> Result<Expr, Diagnostic> {
        use ast::Kind as A;
        let span = expression.span.clone();
        let ty = if let A::Lambda { body, .. } = &expression.kind {
            self.types.get(&body.span)
        } else {
            self.types.get(&span)
        }
        .cloned()
        .ok_or_else(|| Diagnostic::typing(span.clone(), "argument was not checked"))?;
        let boxed = |value| self.expression(value).map(Box::new);
        let kind = match &expression.kind {
            A::List(values) | A::Set(values) => {
                let values = values
                    .iter()
                    .map(|value| self.expression(value))
                    .collect::<Result<_, _>>()?;
                if matches!(expression.kind, A::List(_)) {
                    Kind::List(values)
                } else {
                    Kind::Set(values)
                }
            }
            A::Construct { arguments, .. } => Kind::Construct(self.arguments(arguments)?),
            A::Int(value) => Kind::Int(*value),
            A::Float(value) => Kind::Float(*value),
            A::Bool(value) => Kind::Bool(*value),
            A::Str(value) => Kind::Str(value.clone()),
            A::Ident(name) => Kind::Ident(name.clone()),
            A::Cards => Kind::Cards,
            A::DocRef(name) => Kind::DocRef(name.clone()),
            A::Data(entries) => Kind::Data(
                entries
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), self.expression(value)?)))
                    .collect::<Result<_, Diagnostic>>()?,
            ),
            A::Add(..) => {
                let mut first = expression;
                let mut rest = Vec::new();
                while let A::Add(left, right) = &first.kind {
                    rest.push((self.expression(right)?, first.span.clone()));
                    first = left;
                }
                rest.reverse();
                Kind::Add {
                    first: boxed(first)?,
                    rest,
                }
            }
            A::Compare {
                left,
                right,
                negated,
            } => Kind::Compare {
                left: boxed(left)?,
                right: boxed(right)?,
                negated: *negated,
            },
            A::Field(receiver, _) => {
                let receiver = boxed(receiver)?;
                let field = self
                    .fields
                    .get(&span)
                    .cloned()
                    .ok_or_else(|| Diagnostic::name(span.clone(), "unknown checked field"))?;
                Kind::Field(receiver, field)
            }
            A::Link {
                source,
                target,
                data,
            } => Kind::Link {
                source: boxed(source)?,
                target: boxed(target)?,
                data: data.as_deref().map(boxed).transpose()?,
            },
            A::Lambda { parameters, body } => Kind::Lambda {
                parameters: parameters.clone(),
                body: boxed(body)?,
            },
            A::Call { callee, arguments } => {
                if let Some(step) = self.steps.get(&span) {
                    let (input, arguments) = arguments
                        .split_first()
                        .ok_or_else(|| Diagnostic::typing(span.clone(), "step needs an input"))?;
                    Kind::Step {
                        step,
                        input: boxed(&input.value)?,
                        arguments: self.arguments(arguments)?,
                    }
                } else if matches!(&callee.kind, A::Ident(name) if name == "compare") {
                    Kind::Order(boxed(&arguments[0].value)?, boxed(&arguments[1].value)?)
                } else {
                    Kind::Construct(self.arguments(arguments)?)
                }
            }
            A::Pipe(input, written) => {
                let step = self
                    .steps
                    .get(&span)
                    .ok_or_else(|| Diagnostic::typing(span.clone(), "step was not resolved"))?;
                let arguments = match &written.kind {
                    A::Call { arguments, .. } => arguments.as_slice(),
                    _ => &[],
                };
                Kind::Step {
                    step,
                    input: boxed(input)?,
                    arguments: self.arguments(arguments)?,
                }
            }
        };
        Ok(Expr { kind, ty, span })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_owns_its_tree_and_never_resolves_imports_again() {
        let mut vault = crate::vault::Vault::empty();
        let syntax = crate::parser::parse("[1, 2] | map(n => n + 1) | table").unwrap();
        let program = crate::checker::check(&syntax, &vault).unwrap();
        drop(syntax);
        vault.extensions.prelude = false;
        vault.extensions.imports.push("unregistered".to_owned());
        let (value, warnings) = crate::evaluation::evaluate(&program, &vault).unwrap();
        assert_eq!(value.type_of(), program.result);
        assert!(warnings.is_empty());
    }

    #[test]
    fn failed_collecting_checks_never_produce_an_executable_program() {
        let vault = crate::vault::Vault::empty();
        let syntax = crate::parser::parse("[1] | take(true)\n[2] | take(1)").unwrap();
        let (program, errors) = crate::checker::check_collecting(&syntax, &vault);
        assert!(program.is_none());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn checked_execution_can_be_shared_with_a_future_planner() {
        fn shared<T: Send + Sync>() {}
        shared::<Program>();
    }
}
