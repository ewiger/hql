//! Checked execution tree: resolved operations, result types, and source spans.

use crate::ast::{self, Arith, Logic, Relation};
use crate::diagnostics::Diagnostic;
use crate::extensions::Operation;
use crate::types::TypeRef;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

pub(crate) type Arg = ast::Arg<Expr>;

#[derive(Debug)]
pub(crate) struct Program {
    pub statements: Vec<Stmt>,
    pub result: TypeRef,
    /// The warnings checking reported.
    pub warnings: Vec<crate::warnings::Warning>,
    /// Each alias the program defines, with the signature the checker wrote.
    pub signatures: Vec<(String, TypeRef)>,
    /// Each function the program defines whose body is effectful, with the
    /// effectful operations it uses.
    pub effects: Vec<(String, Vec<String>)>,
}

#[derive(Debug)]
pub(crate) enum Stmt {
    Bind {
        name: String,
        view: TypeRef,
        value: Expr,
    },
    Expr(Expr),
    /// A declared function, bound under its name.
    Fn {
        name: String,
        parameters: Vec<String>,
        body: std::sync::Arc<Expr>,
        ty: TypeRef,
    },
    Unit,
}

/// One statement of an iterating function's body.
#[derive(Debug)]
pub(crate) enum BlockStmt {
    Bind {
        name: String,
        value: Expr,
    },
    Yield(Expr),
    For {
        name: String,
        iterable: Expr,
        body: Vec<BlockStmt>,
    },
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
    DocRef(String),
    Data(Vec<(String, Expr)>),
    /// A left-deep chain, flattened so evaluation spends no stack per link.
    /// Each link keeps the span of the partial result it produces.
    Arith {
        first: Box<Expr>,
        rest: Vec<(Arith, Expr, Range<usize>)>,
    },
    Compare {
        left: Box<Expr>,
        right: Box<Expr>,
        negated: bool,
    },
    Relate {
        left: Box<Expr>,
        right: Box<Expr>,
        relation: Relation,
    },
    /// A left-deep chain of connectives, flattened like `Arith`.
    Logic {
        first: Box<Expr>,
        rest: Vec<(Logic, Expr)>,
    },
    Not(Box<Expr>),
    Order(Box<Expr>, Box<Expr>),
    Step {
        step: &'static Operation,
        input: Box<Expr>,
        arguments: Vec<Arg>,
        /// Where the step itself is written: `debug(…)` in `xs | debug(…)`.
        at: Range<usize>,
    },
    Link {
        source: Box<Expr>,
        target: Box<Expr>,
        data: Option<Box<Expr>>,
    },
    Field(Box<Expr>, Field),
    /// `xs[i]`: the element at a position, or its absence.
    Index(Box<Expr>, Box<Expr>),
    Lambda {
        parameters: Vec<String>,
        body: std::sync::Arc<Expr>,
        /// Its type, when it is written as a value rather than as a step's
        /// argument.
        function: Option<TypeRef>,
    },
    Type,
    /// Two values combined: `min(a, b)`, `merge(m, n)`, `union`, `concat`.
    Combine {
        combiner: &'static str,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// An iterating function's body: evaluating it makes a traversal and
    /// runs nothing.
    Generate {
        statements: std::sync::Arc<Vec<BlockStmt>>,
        element: TypeRef,
        finite: bool,
    },
    /// A function value applied to arguments. `pack` collects the arguments
    /// past the first `n` into a list of the given element type, for a
    /// variadic parameter.
    Apply {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        pack: Option<(usize, TypeRef)>,
    },
    /// `some(value)`.
    Some(Box<Expr>),
    /// `input | tee name = branch`: the input, evaluated once, is read by
    /// the branch under [`crate::ast::TEE_INPUT`] and passed on.
    Tee {
        input: Box<Expr>,
        name: String,
        branch: Box<Expr>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<Arm>,
    },
    Given {
        value: Box<Expr>,
        condition: Box<Expr>,
        otherwise: Option<Box<Expr>>,
    },
}

/// A checked `match` arm.
#[derive(Debug)]
pub(crate) struct Arm {
    pub pattern: crate::ast::Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
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
    Value,
    Score,
    Query,
    Retrieval,
    Hits,
    Nodes,
    Edges,
    Keys,
    GroupKey,
    Items,
    Cards,
    Root,
    Graph,
    End,
    Level,
    Logger,
    Message,
    Logs,
    Url,
    Method,
    Headers,
    Host,
    Port,
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
            "value" => Self::Value,
            "score" => Self::Score,
            "query" => Self::Query,
            "retrieval" => Self::Retrieval,
            "hits" => Self::Hits,
            "nodes" => Self::Nodes,
            "edges" => Self::Edges,
            "keys" => Self::Keys,
            "key" => Self::GroupKey,
            "items" => Self::Items,
            "cards" => Self::Cards,
            "root" => Self::Root,
            "graph" => Self::Graph,
            "end" => Self::End,
            "level" => Self::Level,
            "logger" => Self::Logger,
            "message" => Self::Message,
            "logs" => Self::Logs,
            "url" => Self::Url,
            "method" => Self::Method,
            "headers" => Self::Headers,
            "host" => Self::Host,
            "port" => Self::Port,
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
    pub steps: HashMap<Range<usize>, &'static Operation>,
    /// A lambda written as a value, or a declared function, at its type.
    pub functions: HashMap<Range<usize>, TypeRef>,
    /// A call or pipe that applies a function value, with how many leading
    /// arguments it takes before the trailing ones a variadic parameter
    /// collects, and their element type.
    pub applications: HashMap<Range<usize>, Option<(usize, TypeRef)>>,
    /// Each constructor passed as a function, with the type it takes and the
    /// type it builds.
    pub constructor_values: HashMap<Range<usize>, (TypeRef, TypeRef)>,
    /// The calls that combine two values, by the combiner they name.
    pub combiners: HashMap<Range<usize>, &'static str>,
    /// What checking found worth saying that does not make the program wrong.
    pub warnings: Vec<crate::warnings::Warning>,
    /// Each alias the program defines, with the signature the checker wrote.
    pub signatures: Vec<(String, TypeRef)>,
    /// Each effectful function, with the effectful operations it uses.
    pub effects: Vec<(String, Vec<String>)>,
}

impl Checked {
    pub fn program(&self, program: &ast::Program, result: TypeRef) -> Result<Program, Diagnostic> {
        let statements =
            program
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
                        ast::Stmt::Alias(alias) => Stmt::Fn {
                            name: alias.name.clone(),
                            parameters: alias
                                .parameters
                                .iter()
                                .map(|parameter| parameter.name.clone())
                                .collect(),
                            body: std::sync::Arc::new(self.expression(&alias.body)?),
                            ty: self.functions.get(&alias.span).cloned().ok_or_else(|| {
                                Diagnostic::typing(alias.span.clone(), "alias was not checked")
                            })?,
                        },
                        ast::Stmt::Fn(declaration) => Stmt::Fn {
                            name: declaration.name.clone(),
                            parameters: declaration
                                .parameters
                                .iter()
                                .map(|parameter| parameter.name.clone())
                                .collect(),
                            body: std::sync::Arc::new(self.expression(&declaration.body)?),
                            ty: self.functions.get(&declaration.span).cloned().ok_or_else(
                                || {
                                    Diagnostic::typing(
                                        declaration.span.clone(),
                                        "function was not checked",
                                    )
                                },
                            )?,
                        },
                        ast::Stmt::Import { .. } | ast::Stmt::Type(_) => Stmt::Unit,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?;
        Ok(Program {
            statements,
            result,
            warnings: self.warnings.clone(),
            signatures: self.signatures.clone(),
            effects: self.effects.clone(),
        })
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

    fn block(&self, statements: &[ast::BlockStmt]) -> Result<Vec<BlockStmt>, Diagnostic> {
        statements
            .iter()
            .map(|statement| {
                Ok(match statement {
                    ast::BlockStmt::Bind { name, value } => BlockStmt::Bind {
                        name: name.clone(),
                        value: self.expression(value)?,
                    },
                    ast::BlockStmt::Yield(value) => BlockStmt::Yield(self.expression(value)?),
                    ast::BlockStmt::For {
                        name,
                        iterable,
                        body,
                    } => BlockStmt::For {
                        name: name.clone(),
                        iterable: self.expression(iterable)?,
                        body: self.block(body)?,
                    },
                })
            })
            .collect()
    }

    fn expression(&self, expression: &ast::Expr) -> Result<Expr, Diagnostic> {
        use ast::Kind as A;
        let span = expression.span.clone();
        let ty = if let Some(function) = self.functions.get(&span) {
            Some(function)
        } else if let A::Lambda { body, .. } = &expression.kind {
            self.types.get(&body.span)
        } else {
            self.types.get(&span)
        }
        .cloned()
        .ok_or_else(|| Diagnostic::typing(span.clone(), "argument was not checked"))?;
        let boxed = |value| self.expression(value).map(Box::new);
        let kind = match &expression.kind {
            A::Block(statements) => Kind::Generate {
                statements: std::sync::Arc::new(self.block(statements)?),
                element: ty.args.first().cloned().unwrap_or(TypeRef::NEVER),
                finite: ty.constructor == crate::types::collections::SEQ,
            },
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
            A::Ident(name) if self.constructor_values.contains_key(&span) => {
                let (parameter, built) = self.constructor_values[&span].clone();
                let held = Expr {
                    kind: Kind::Ident(crate::checker::CONSTRUCTED.to_owned()),
                    ty: parameter.clone(),
                    span: span.clone(),
                };
                let _ = name;
                Kind::Lambda {
                    parameters: vec![crate::checker::CONSTRUCTED.to_owned()],
                    body: std::sync::Arc::new(Expr {
                        kind: Kind::Construct(vec![Arg {
                            name: None,
                            value: held,
                        }]),
                        ty: built.clone(),
                        span: span.clone(),
                    }),
                    function: TypeRef::function(vec![parameter], built),
                }
            }
            A::Ident(name) => Kind::Ident(name.clone()),
            A::DocRef(name) => Kind::DocRef(name.clone()),
            A::Data(entries) => Kind::Data(
                entries
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), self.expression(value)?)))
                    .collect::<Result<_, Diagnostic>>()?,
            ),
            A::Arith(..) => {
                let mut first = expression;
                let mut rest = Vec::new();
                while let A::Arith(operator, left, right) = &first.kind {
                    rest.push((*operator, self.expression(right)?, first.span.clone()));
                    first = left;
                }
                rest.reverse();
                Kind::Arith {
                    first: boxed(first)?,
                    rest,
                }
            }
            A::Relate {
                left,
                right,
                relation,
            } => Kind::Relate {
                left: boxed(left)?,
                right: boxed(right)?,
                relation: *relation,
            },
            A::Logic(..) => {
                let mut first = expression;
                let mut rest = Vec::new();
                while let A::Logic(connective, left, right) = &first.kind {
                    rest.push((*connective, self.expression(right)?));
                    first = left;
                }
                rest.reverse();
                Kind::Logic {
                    first: boxed(first)?,
                    rest,
                }
            }
            A::Not(operand) => Kind::Not(boxed(operand)?),
            A::Compare {
                left,
                right,
                negated,
            } => Kind::Compare {
                left: boxed(left)?,
                right: boxed(right)?,
                negated: *negated,
            },
            A::Index(receiver, index) => Kind::Index(boxed(receiver)?, boxed(index)?),
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
            A::Lambda {
                parameters, body, ..
            } => Kind::Lambda {
                parameters: parameters.clone(),
                body: std::sync::Arc::new(self.expression(body)?),
                function: self.functions.get(&span).cloned(),
            },
            A::Call { arguments, .. } if self.combiners.contains_key(&span) => Kind::Combine {
                combiner: self.combiners[&span],
                left: Box::new(self.expression(&arguments[0].value)?),
                right: Box::new(self.expression(&arguments[1].value)?),
            },
            A::Call { callee, arguments } if self.applications.contains_key(&span) => Kind::Apply {
                callee: boxed(callee)?,
                arguments: arguments
                    .iter()
                    .map(|argument| self.expression(&argument.value))
                    .collect::<Result<_, _>>()?,
                pack: self.applications[&span].clone(),
            },
            A::Pipe(input, written) if matches!(written.kind, A::Tee { .. }) => {
                let A::Tee { name, branch } = &written.kind else {
                    unreachable!("matched above");
                };
                Kind::Tee {
                    input: boxed(input)?,
                    name: name.clone(),
                    branch: boxed(branch)?,
                }
            }
            A::Tee { .. } => {
                return Err(Diagnostic::syntax(span, "`tee` is a pipeline step"));
            }
            A::Pipe(input, written) if self.applications.contains_key(&span) => {
                let (callee, rest) = match &written.kind {
                    A::Call { callee, arguments } => (callee.as_ref(), arguments.as_slice()),
                    _ => (written.as_ref(), &[][..]),
                };
                let mut arguments = vec![self.expression(input)?];
                for argument in rest {
                    arguments.push(self.expression(&argument.value)?);
                }
                Kind::Apply {
                    callee: boxed(callee)?,
                    arguments,
                    pack: self.applications[&span].clone(),
                }
            }
            A::Match { subject, arms } => Kind::Match {
                subject: boxed(subject)?,
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(Arm {
                            pattern: arm.pattern.clone(),
                            guard: arm
                                .guard
                                .as_ref()
                                .map(|guard| self.expression(guard))
                                .transpose()?,
                            body: self.expression(&arm.body)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            A::Given {
                value,
                condition,
                otherwise,
            } => Kind::Given {
                value: boxed(value)?,
                condition: boxed(condition)?,
                otherwise: otherwise.as_deref().map(boxed).transpose()?,
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
                        at: span.clone(),
                    }
                } else if matches!(&callee.kind, A::Ident(name) if name == "some") {
                    Kind::Some(boxed(&arguments[0].value)?)
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
                    at: written.span.clone(),
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
        let syntax = crate::parser::parse("[1, 2] | map(n => n + 1) | markdown").unwrap();
        let program = crate::checker::check(&syntax, &vault).unwrap();
        drop(syntax);
        vault.extensions.prelude = false;
        vault.extensions.imports.push("unregistered".to_owned());
        let (value, log) = crate::evaluation::evaluate(&program, &vault);
        assert_eq!(value.unwrap().type_of(), program.result);
        assert!(log.is_empty());
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
