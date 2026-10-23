//! Checked execution tree: resolved operations, result types, and source spans.

use crate::ast::{self, Arith, Logic, Relation};
use crate::diagnostics::Diagnostic;
use crate::extensions::{Operation, Purity, Shape};
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
    /// The lattice the program was checked under: the built-in types and the
    /// ones it declared, which evaluation reads its values against.
    pub lattice: std::sync::Arc<crate::types::program::Lattice>,
    /// The workspace modules the program imported, in import order, each
    /// evaluated before the program for the functions it binds.
    pub modules: Vec<LoadedModule>,
}

/// A workspace module bound into a program under a local name.
#[derive(Debug, Clone)]
pub(crate) struct LoadedModule {
    pub local: String,
    pub program: std::sync::Arc<Program>,
}

#[derive(Debug)]
pub(crate) enum Stmt {
    Bind {
        name: String,
        view: TypeRef,
        value: Expr,
    },
    Expr(Expr),
    /// A contract's rules, each run over the fields of every value read as
    /// the contract.
    Contract {
        name: String,
        rules: Vec<RuleLowering>,
    },
    /// A declared function, bound under its name.
    Fn {
        name: String,
        parameters: Vec<String>,
        body: std::sync::Arc<Expr>,
        ty: TypeRef,
    },
    /// An implementation of a trait for a type: each function, `self` first
    /// among its parameters, as a closure over the scope at this point.
    Impl {
        trait_name: String,
        target: TypeRef,
        methods: Vec<(String, Vec<String>, std::sync::Arc<Expr>, TypeRef)>,
    },
    Unit,
}

/// What checking established about one implementation, for lowering it.
#[derive(Debug, Clone)]
pub(crate) struct ImplLowering {
    pub trait_name: String,
    pub target: TypeRef,
    pub methods: Vec<MethodLowering>,
}

/// A name a contract rule binds, and what of the value it binds.
#[derive(Debug, Clone)]
pub(crate) struct RuleParameter {
    pub name: String,
    pub field: String,
    pub reads: RuleRead,
}

/// What a rule parameter holds of its field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleRead {
    /// The field's value as declared: an optional one as an `Option`.
    Value,
    /// An optional field's value where present, written `f?`: a rule that
    /// reads one that is absent does not apply.
    Present,
    /// Whether an optional field is present, `f?` standing as a condition.
    Presence,
}

/// A contract rule lowered for evaluation: a function of its parameters.
#[derive(Debug)]
pub(crate) struct RuleLowering {
    pub text: String,
    pub severity: ast::Severity,
    pub name: Option<String>,
    /// The optional fields it reads as `f?`: it applies where all are
    /// present.
    pub reads: Vec<String>,
    pub parameters: Vec<RuleParameter>,
    pub body: std::sync::Arc<Expr>,
}

/// One function of an implementation: its own, or inherited from the trait.
#[derive(Debug, Clone)]
pub(crate) struct MethodLowering {
    pub name: String,
    /// `self` first, then the parameters.
    pub parameters: Vec<String>,
    /// The trait whose default body this is, when the impl did not write one.
    pub default_of: Option<String>,
    pub ty: TypeRef,
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
    Decimal(crate::decimal::Decimal),
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
        /// Whether the step is lifted over an optional input it takes no
        /// collection of: absence gives absence, and a value it produces
        /// is present.
        lifted: bool,
        /// The declaration the call resolved to, whichever of a call, a
        /// receiver call or a pipeline step wrote it: none for an operation
        /// of a module without a contract. The evaluator dispatches on the
        /// operation bound to it; a planner reads the declaration.
        #[cfg_attr(
            not(test),
            expect(dead_code, reason = "read by a planner, not by the evaluator")
        )]
        declaration: Option<crate::extensions::binding::DeclarationId>,
        /// What the planner reads of the step and the stage it writes.
        plan: StepPlan,
    },
    Link {
        source: Box<Expr>,
        target: Box<Expr>,
        data: Option<Box<Expr>>,
    },
    Field(Box<Expr>, Field),
    /// `xs[i]`: the element at a position, or its absence.
    Index(Box<Expr>, Box<Expr>),
    /// `xs[a:b]`: the part between two positions, either one left out.
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>),
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
        /// Present when the application is a pipeline step, `x | f`: the
        /// input is the first argument.
        plan: Option<StepPlan>,
    },
    /// `some(value)`.
    Some(Box<Expr>),
    /// `ok(value)`: a result that succeeded.
    Ok(Box<Expr>),
    /// `err(error)`: a result that failed.
    Err(Box<Expr>),
    /// `Point(x = 1, y = 2)`: a record of the expression's type, from the
    /// fields written, each by name.
    Record(Vec<(String, Expr)>),
    /// `data | User`: the input read as a declared type, each `Data` of it
    /// where it is optional or a collection. Typing a value changes what is
    /// known of it, not the value: an open record keeps the whole tree.
    Interpret {
        input: Box<Expr>,
        element: TypeRef,
        /// `x | validate(T)`: each read gives a `Result<T, Invalid>`
        /// (HQL-0033).
        validated: bool,
    },
    /// A trait's function called on a receiver, `v.area()` or `a + b`,
    /// dispatched at run time on the receiver's type.
    Method {
        trait_name: String,
        method: String,
        receiver: Box<Expr>,
        arguments: Vec<Expr>,
    },
    /// `input | tee name = branch`: the input, evaluated once, is read by
    /// the branch under [`crate::ast::TEE_INPUT`] and passed on.
    Tee {
        input: Box<Expr>,
        name: String,
        branch: Box<Expr>,
        plan: StepPlan,
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

/// What a step invokes, as a planner reads it: the operation's name, its
/// shape and its purity, and whether it presents its input. A host
/// operation and a program's own function reach the step node through this
/// one type, so the planner has one code path for both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationType {
    pub name: String,
    pub shape: Shape,
    pub purity: Purity,
    /// Whether the operation is a presentation: `format` and its shorthands.
    pub presentation: bool,
}

impl OperationType {
    /// A host operation, at the shape its arguments refine it to.
    pub(crate) fn host(step: &'static Operation, arguments: &[Arg]) -> Self {
        Self {
            name: step.name.to_owned(),
            shape: shape_of(step, arguments),
            purity: step.purity(),
            presentation: crate::extensions::format::STEPS
                .iter()
                .any(|presentation| std::ptr::eq(presentation, step)),
        }
    }
}

/// The planner's field on a step node: what the step invokes, and the stage
/// annotation, empty until the planner fills it.
#[derive(Debug)]
pub(crate) struct StepPlan {
    pub invokes: OperationType,
    pub stage: std::sync::OnceLock<crate::backends::Stage>,
}

impl StepPlan {
    pub(crate) fn new(invokes: OperationType) -> Self {
        Self {
            invokes,
            stage: std::sync::OnceLock::new(),
        }
    }
}

impl Expr {
    /// The step node this expression is, with its input: a host
    /// operation's step, a function applied by `|`, or a `tee`.
    pub(crate) fn as_step(&self) -> Option<(&StepPlan, &Expr)> {
        match &self.kind {
            Kind::Step { plan, input, .. } | Kind::Tee { plan, input, .. } => Some((plan, input)),
            Kind::Apply {
                plan: Some(plan),
                arguments,
                ..
            } => arguments.first().map(|input| (plan, input)),
            _ => None,
        }
    }

    /// The step nodes of the pipeline this expression ends, first to last,
    /// and the expression the first one reads. Expressions inside a step's
    /// arguments, a lambda's body among them, are evaluated within that
    /// step and are not steps of the pipeline.
    pub(crate) fn steps(&self) -> (Vec<&StepPlan>, &Expr) {
        let mut steps = Vec::new();
        let mut at = self;
        while let Some((plan, input)) = at.as_step() {
            steps.push(plan);
            at = input;
        }
        steps.reverse();
        (steps, at)
    }

    /// The expressions this one holds directly, in the order they are
    /// written.
    pub(crate) fn children(&self) -> Vec<&Expr> {
        match &self.kind {
            Kind::Int(_)
            | Kind::Float(_)
            | Kind::Decimal(_)
            | Kind::Bool(_)
            | Kind::Str(_)
            | Kind::Ident(_)
            | Kind::DocRef(_)
            | Kind::Type => Vec::new(),
            Kind::List(values) | Kind::Set(values) => values.iter().collect(),
            Kind::Construct(held) => argument_values(held),
            Kind::Data(entries) => entries.iter().map(|(_, value)| value).collect(),
            Kind::Record(fields) => fields.iter().map(|(_, value)| value).collect(),
            Kind::Interpret { input, .. } => vec![input],
            Kind::Arith { first, rest } => std::iter::once(first.as_ref())
                .chain(rest.iter().map(|(_, operand, _)| operand))
                .collect(),
            Kind::Logic { first, rest } => std::iter::once(first.as_ref())
                .chain(rest.iter().map(|(_, operand)| operand))
                .collect(),
            Kind::Compare { left, right, .. }
            | Kind::Relate { left, right, .. }
            | Kind::Order(left, right)
            | Kind::Index(left, right)
            | Kind::Combine { left, right, .. } => vec![left, right],
            Kind::Not(inner)
            | Kind::Field(inner, _)
            | Kind::Some(inner)
            | Kind::Ok(inner)
            | Kind::Err(inner) => vec![inner],
            Kind::Slice(value, start, end) => std::iter::once(value.as_ref())
                .chain(start.as_deref())
                .chain(end.as_deref())
                .collect(),
            Kind::Step {
                input,
                arguments: held,
                ..
            } => std::iter::once(input.as_ref())
                .chain(argument_values(held))
                .collect(),
            Kind::Link {
                source,
                target,
                data,
            } => [source.as_ref(), target.as_ref()]
                .into_iter()
                .chain(data.as_deref())
                .collect(),
            Kind::Lambda { body, .. } => vec![body.as_ref()],
            Kind::Generate { statements, .. } => {
                let mut held = Vec::new();
                block_expressions(statements, &mut held);
                held
            }
            Kind::Apply {
                callee, arguments, ..
            } => std::iter::once(callee.as_ref()).chain(arguments).collect(),
            Kind::Method {
                receiver,
                arguments,
                ..
            } => std::iter::once(receiver.as_ref())
                .chain(arguments)
                .collect(),
            Kind::Tee { input, branch, .. } => vec![input, branch],
            Kind::Match { subject, arms } => std::iter::once(subject.as_ref())
                .chain(
                    arms.iter()
                        .flat_map(|arm| arm.guard.iter().chain(std::iter::once(&arm.body))),
                )
                .collect(),
            Kind::Given {
                value,
                condition,
                otherwise,
            } => [value.as_ref(), condition.as_ref()]
                .into_iter()
                .chain(otherwise.as_deref())
                .collect(),
        }
    }

    /// Every pipeline under this expression, each by the step node that
    /// ends it, outermost first. A pipeline inside a step's arguments, a
    /// lambda's body among them, is a pipeline of its own.
    pub(crate) fn pipelines<'e>(&'e self, found: &mut Vec<&'e Expr>) {
        if self.as_step().is_none() {
            for child in self.children() {
                child.pipelines(found);
            }
            return;
        }
        found.push(self);
        let mut at = self;
        while let Some((_, input)) = at.as_step() {
            for child in at.children() {
                if !std::ptr::eq(child, input) {
                    child.pipelines(found);
                }
            }
            at = input;
        }
        at.pipelines(found);
    }

    /// Whether a step under this expression is effectful.
    pub(crate) fn reaches_effect(&self) -> bool {
        let mut found = Vec::new();
        self.pipelines(&mut found);
        found.iter().any(|end| {
            end.steps()
                .0
                .iter()
                .any(|plan| plan.invokes.purity == Purity::Effectful)
        })
    }

    /// The steps of the pipeline this expression ends, first to last, each
    /// with the shape it was checked to.
    pub(crate) fn pipeline(&self) -> Vec<(String, Shape)> {
        self.steps()
            .0
            .into_iter()
            .map(|plan| (plan.invokes.name.clone(), plan.invokes.shape))
            .collect()
    }
}

/// The shape of a checked step: its operation's, refined by a combiner. A
/// fold whose `by` names an associative combiner the host holds is a
/// `Reduction`, since its splits may be folded and combined; a fold with a
/// lambda stays the barrier `reduce` is declared, because nothing is known
/// about the lambda.
pub(crate) fn shape_of(step: &Operation, arguments: &[Arg]) -> Shape {
    let declared = step.shape();
    if step.name != "reduce" {
        return declared;
    }
    let named = arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("by"))
        .or_else(|| arguments.last());
    match named.and_then(known_combiner) {
        Some(combiner) if combiner.associative => Shape::Reduction,
        _ => declared,
    }
}

/// The combiner an argument names directly, `+` in `reduce(0, +)`: the
/// lambda the parser writes for it, whose parameters no program can name.
pub(crate) fn known_combiner(argument: &Arg) -> Option<&'static crate::extensions::Combiner> {
    let Kind::Lambda {
        parameters, body, ..
    } = &argument.value.kind
    else {
        return None;
    };
    if !parameters
        .first()
        .is_some_and(|parameter| parameter.starts_with('%'))
    {
        return None;
    }
    let name = match &body.kind {
        Kind::Combine { combiner, .. } => combiner,
        Kind::Arith { rest, .. } => match rest.first().map(|(operator, _, _)| operator) {
            Some(Arith::Add) => "+",
            Some(Arith::Mul) => "*",
            _ => return None,
        },
        Kind::Method { trait_name, .. } => match trait_name.as_str() {
            "Add" => "+",
            "Mul" => "*",
            _ => return None,
        },
        _ => return None,
    };
    crate::extensions::combiner(name)
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
    /// A field of a declared record, by name.
    Record(String),
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
        if crate::types::program::is_record_valued(receiver.present().constructor.0) {
            return Some(Self::Record(name.to_owned()));
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
    /// The steps lifted over an optional input, absence giving absence.
    pub lifted: HashSet<Range<usize>>,
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
    /// The calls that construct a record, at the type they build.
    pub records: HashMap<Range<usize>, TypeRef>,
    /// The declaration each step call resolved to, by where the step is
    /// written.
    pub declarations: HashMap<Range<usize>, crate::extensions::binding::DeclarationId>,
    /// The calls written `receiver.f(…)`, whose receiver is the input.
    pub receiver_calls: HashSet<Range<usize>>,
    /// Each step named as a function value, `map(str)`, with the type it
    /// takes and the type it produces.
    pub step_values: HashMap<Range<usize>, (&'static Operation, TypeRef, TypeRef)>,
    /// Each pipeline step whose operation is a function value, `x | f`, by
    /// the pipe's span: the operation type the planner reads, and whether
    /// an `Elementwise` runs over each element of its input.
    pub user_steps: HashMap<Range<usize>, (OperationType, bool)>,
    /// Each pipe that reads its input as a declared type, `data | User`, by
    /// the pipe's span: the type each `Data` is read as (HQL-0030).
    pub interpretations: HashMap<Range<usize>, TypeRef>,
    /// Each pipe that validates its input as a declared type,
    /// `data | validate(User)`, by the pipe's span: the type each value is
    /// read as (HQL-0033).
    pub validations: HashMap<Range<usize>, TypeRef>,
    /// Each call of a trait's function, by the trait and the function.
    pub method_calls: HashMap<Range<usize>, (String, String)>,
    /// Each operator resolved through a trait, by the trait.
    pub operator_calls: HashMap<Range<usize>, &'static str>,
    /// Each implementation, by the span of its declaration.
    pub impls: HashMap<Range<usize>, ImplLowering>,
    /// Each contract with rules, by the span of its declaration: the names a
    /// rule's body reads its fields by, in the order a check binds them.
    pub contracts: HashMap<Range<usize>, Vec<RuleParameter>>,
    /// Each reference to a workspace module's function, `geo.area`, by the
    /// name it is bound under.
    pub module_refs: HashMap<Range<usize>, String>,
    /// Each function bound under another name than it is declared with: a
    /// program's constructor, one name per form, by its declaration's span.
    pub bound_names: HashMap<Range<usize>, String>,
    /// Each receiver call of a workspace module's function, `"hi".shout()`,
    /// by the name it is bound under and its type.
    pub module_calls: HashMap<Range<usize>, (String, TypeRef)>,
    /// The workspace modules imported, in order.
    pub modules: Vec<LoadedModule>,
    /// What checking found worth saying that does not make the program wrong.
    pub warnings: Vec<crate::warnings::Warning>,
    /// Each alias the program defines, with the signature the checker wrote.
    pub signatures: Vec<(String, TypeRef)>,
    /// Each effectful function, with the effectful operations it uses.
    pub effects: Vec<(String, Vec<String>)>,
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
                    ast::Stmt::Fn(declaration) => {
                        Stmt::Fn {
                            name: self
                                .bound_names
                                .get(&declaration.span)
                                .cloned()
                                .unwrap_or_else(|| declaration.name.clone()),
                            parameters: declaration
                                .has_self
                                .then(|| "self".to_owned())
                                .into_iter()
                                .chain(
                                    declaration
                                        .parameters
                                        .iter()
                                        .map(|parameter| parameter.name.clone()),
                                )
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
                        }
                    }
                    ast::Stmt::Type(declaration) => match self.contracts.get(&declaration.span) {
                        Some(parameters) => Stmt::Contract {
                            name: declaration.name.clone(),
                            rules: declaration
                                .rules
                                .iter()
                                .map(|rule| {
                                    Ok(RuleLowering {
                                        text: rule.text.clone(),
                                        severity: rule.severity,
                                        name: rule.name.clone(),
                                        reads: rule.reads.clone(),
                                        parameters: parameters.clone(),
                                        body: std::sync::Arc::new(
                                            self.expression(&rule.predicate)?,
                                        ),
                                    })
                                })
                                .collect::<Result<_, Diagnostic>>()?,
                        },
                        None => Stmt::Unit,
                    },
                    ast::Stmt::Import { .. }
                    | ast::Stmt::ModuleFor { .. }
                    | ast::Stmt::External(_)
                    | ast::Stmt::Trait(_) => Stmt::Unit,
                    // A native implementation runs no HQL: its header is
                    // bound to an extension's bodies.
                    ast::Stmt::Impl(declaration) if declaration.native => Stmt::Unit,
                    ast::Stmt::Impl(declaration) => {
                        let lowering = self.impls.get(&declaration.span).ok_or_else(|| {
                            Diagnostic::typing(
                                declaration.span.clone(),
                                "implementation was not checked",
                            )
                        })?;
                        let mut methods = Vec::new();
                        for method in &lowering.methods {
                            let body = match &method.default_of {
                                None => declaration
                                    .methods
                                    .iter()
                                    .find(|held| held.name == method.name)
                                    .and_then(|held| held.body.as_ref()),
                                Some(trait_name) => program
                                    .statements
                                    .iter()
                                    .find_map(|statement| match statement {
                                        ast::Stmt::Trait(held) if held.name == *trait_name => {
                                            Some(held)
                                        }
                                        _ => None,
                                    })
                                    .and_then(|held| {
                                        held.methods.iter().find(|held| held.name == method.name)
                                    })
                                    .and_then(|held| held.body.as_ref()),
                            };
                            let body = body.ok_or_else(|| {
                                Diagnostic::typing(
                                    declaration.span.clone(),
                                    format!("`{}` has no body to run", method.name),
                                )
                            })?;
                            methods.push((
                                method.name.clone(),
                                method.parameters.clone(),
                                std::sync::Arc::new(self.expression(body)?),
                                method.ty.clone(),
                            ));
                        }
                        Stmt::Impl {
                            trait_name: lowering.trait_name.clone(),
                            target: lowering.target.clone(),
                            methods,
                        }
                    }
                })
            })
            .collect::<Result<_, Diagnostic>>()?;
        Ok(Program {
            statements,
            result,
            warnings: self.warnings.clone(),
            signatures: self.signatures.clone(),
            effects: self.effects.clone(),
            lattice: crate::types::program::current()
                .map_err(|error| Diagnostic::typing(0..0, error.to_string()))?,
            modules: self.modules.clone(),
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
            A::Decimal(value) => Kind::Decimal(value.clone()),
            A::Bool(value) => Kind::Bool(*value),
            A::Str(value) => Kind::Str(value.clone()),
            A::Ident(_) if self.module_refs.contains_key(&span) => {
                Kind::Ident(self.module_refs[&span].clone())
            }
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
            A::Ident(_) if self.step_values.contains_key(&span) => {
                let (step, parameter, produced) = self.step_values[&span].clone();
                let held = Expr {
                    kind: Kind::Ident(crate::checker::CONSTRUCTED.to_owned()),
                    ty: parameter.clone(),
                    span: span.clone(),
                };
                Kind::Lambda {
                    parameters: vec![crate::checker::CONSTRUCTED.to_owned()],
                    body: std::sync::Arc::new(Expr {
                        kind: step_node(
                            step,
                            Box::new(held),
                            Vec::new(),
                            span.clone(),
                            self.lifted.contains(&span),
                            self.declarations.get(&span).copied(),
                        ),
                        ty: produced.clone(),
                        span: span.clone(),
                    }),
                    function: TypeRef::function(vec![parameter], produced),
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
            A::Arith(operator, left, right) if self.operator_calls.contains_key(&span) => {
                let (trait_name, method) = match operator {
                    Arith::Add => ("Add", "add"),
                    Arith::Sub => ("Sub", "sub"),
                    Arith::Mul => ("Mul", "mul"),
                    Arith::Div => ("Div", "div"),
                    Arith::Rem => ("Rem", "rem"),
                };
                Kind::Method {
                    trait_name: trait_name.to_owned(),
                    method: method.to_owned(),
                    receiver: boxed(left)?,
                    arguments: vec![self.expression(right)?],
                }
            }
            A::Compare {
                left,
                right,
                negated,
            } if self.operator_calls.contains_key(&span) => {
                let equal = Expr {
                    kind: Kind::Method {
                        trait_name: "Eq".to_owned(),
                        method: "eq".to_owned(),
                        receiver: boxed(left)?,
                        arguments: vec![self.expression(right)?],
                    },
                    ty: TypeRef::BOOL,
                    span: span.clone(),
                };
                if *negated {
                    Kind::Not(Box::new(equal))
                } else {
                    equal.kind
                }
            }
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
            A::Slice(receiver, start, end) => Kind::Slice(
                boxed(receiver)?,
                start.as_deref().map(boxed).transpose()?,
                end.as_deref().map(boxed).transpose()?,
            ),
            A::Field(_, _) if self.module_refs.contains_key(&span) => {
                Kind::Ident(self.module_refs[&span].clone())
            }
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
            A::Call { callee, arguments } if self.module_calls.contains_key(&span) => {
                let A::Field(receiver, _) = &callee.kind else {
                    return Err(Diagnostic::typing(span, "a receiver call reads a receiver"));
                };
                let (key, function) = self.module_calls[&span].clone();
                let mut values = vec![self.expression(receiver)?];
                for argument in arguments {
                    values.push(self.expression(&argument.value)?);
                }
                Kind::Apply {
                    callee: Box::new(Expr {
                        kind: Kind::Ident(key),
                        ty: function,
                        span: callee.span.clone(),
                    }),
                    arguments: values,
                    pack: self.applications.get(&span).cloned().flatten(),
                    plan: None,
                }
            }
            A::Call { callee, arguments } if self.method_calls.contains_key(&span) => {
                let A::Field(receiver, _) = &callee.kind else {
                    return Err(Diagnostic::typing(span, "a method call reads a receiver"));
                };
                let (trait_name, method) = self.method_calls[&span].clone();
                Kind::Method {
                    trait_name,
                    method,
                    receiver: boxed(receiver)?,
                    arguments: arguments
                        .iter()
                        .map(|argument| self.expression(&argument.value))
                        .collect::<Result<_, _>>()?,
                }
            }
            A::Call { callee, arguments } if self.receiver_calls.contains(&span) => {
                let A::Field(receiver, _) = &callee.kind else {
                    return Err(Diagnostic::typing(span, "a receiver call reads a receiver"));
                };
                if let Some(combiner) = self.combiners.get(&span) {
                    Kind::Combine {
                        combiner,
                        left: boxed(receiver)?,
                        right: Box::new(self.expression(&arguments[0].value)?),
                    }
                } else if let Some(step) = self.steps.get(&span) {
                    step_node(
                        step,
                        boxed(receiver)?,
                        self.arguments(arguments)?,
                        span.clone(),
                        self.lifted.contains(&span),
                        self.declarations.get(&span).copied(),
                    )
                } else {
                    Kind::Order(
                        boxed(receiver)?,
                        Box::new(self.expression(&arguments[0].value)?),
                    )
                }
            }
            A::Call { arguments, .. } if self.combiners.contains_key(&span) => Kind::Combine {
                combiner: self.combiners[&span],
                left: Box::new(self.expression(&arguments[0].value)?),
                right: Box::new(self.expression(&arguments[1].value)?),
            },
            A::Call { arguments, .. } if self.interpretations.contains_key(&span) => {
                Kind::Interpret {
                    input: Box::new(self.expression(&arguments[0].value)?),
                    element: self.interpretations[&span].clone(),
                    validated: false,
                }
            }
            A::Call { arguments, .. } if self.records.contains_key(&span) => {
                let shape = crate::types::program::record(self.records[&span].constructor.0)
                    .ok_or_else(|| Diagnostic::typing(span.clone(), "record was not declared"))?;
                let mut fields = Vec::new();
                for (index, argument) in arguments.iter().enumerate() {
                    let name = match &argument.name {
                        Some(name) => name.clone(),
                        None => shape
                            .fields
                            .get(index)
                            .map(|field| field.name.clone())
                            .ok_or_else(|| {
                                Diagnostic::typing(span.clone(), "field out of range")
                            })?,
                    };
                    fields.push((name, self.expression(&argument.value)?));
                }
                Kind::Record(fields)
            }
            A::Call { callee, arguments } if self.applications.contains_key(&span) => Kind::Apply {
                callee: boxed(callee)?,
                arguments: arguments
                    .iter()
                    .map(|argument| self.expression(&argument.value))
                    .collect::<Result<_, _>>()?,
                pack: self.applications[&span].clone(),
                plan: None,
            },
            A::Pipe(input, _) if self.validations.contains_key(&span) => Kind::Interpret {
                input: Box::new(self.expression(input)?),
                element: self.validations[&span].clone(),
                validated: true,
            },
            A::Pipe(input, written) if matches!(written.kind, A::Tee { .. }) => {
                let A::Tee { name, branch } = &written.kind else {
                    unreachable!("matched above");
                };
                let branch = boxed(branch)?;
                let purity = if branch.reaches_effect() {
                    Purity::Effectful
                } else {
                    Purity::Pure
                };
                Kind::Tee {
                    input: boxed(input)?,
                    name: name.clone(),
                    branch,
                    plan: StepPlan::new(OperationType {
                        name: "tee".to_owned(),
                        shape: Shape::Barrier,
                        purity,
                        presentation: false,
                    }),
                }
            }
            A::Tee { .. } => {
                return Err(Diagnostic::syntax(span, "`tee` is a pipeline step"));
            }
            A::Pipe(input, written)
                if self.user_steps.get(&span).is_some_and(|(_, each)| *each) =>
            {
                // An `Elementwise<I, O>` over a collection of `I` is the
                // step `map` runs, invoking the program's operation.
                let (invokes, _) = self.user_steps[&span].clone();
                let map = crate::extensions::collections::STEPS
                    .iter()
                    .find(|step| step.name == "map")
                    .ok_or_else(|| Diagnostic::typing(span.clone(), "`map` is not registered"))?;
                Kind::Step {
                    step: map,
                    input: boxed(input)?,
                    arguments: vec![Arg {
                        name: None,
                        value: self.expression(written)?,
                    }],
                    at: written.span.clone(),
                    lifted: false,
                    declaration: None,
                    plan: StepPlan::new(invokes),
                }
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
                let invokes = self.user_steps.get(&span).map_or_else(
                    || OperationType {
                        name: "<function>".to_owned(),
                        shape: Shape::Barrier,
                        purity: Purity::Effectful,
                        presentation: false,
                    },
                    |(invokes, _)| invokes.clone(),
                );
                Kind::Apply {
                    callee: boxed(callee)?,
                    arguments,
                    pack: self.applications[&span].clone(),
                    plan: Some(StepPlan::new(invokes)),
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
                    step_node(
                        step,
                        boxed(&input.value)?,
                        self.arguments(arguments)?,
                        span.clone(),
                        self.lifted.contains(&span),
                        self.declarations.get(&span).copied(),
                    )
                } else if matches!(&callee.kind, A::Ident(name) if name == "some") {
                    Kind::Some(boxed(&arguments[0].value)?)
                } else if matches!(&callee.kind, A::Ident(name) if name == "ok") {
                    Kind::Ok(boxed(&arguments[0].value)?)
                } else if matches!(&callee.kind, A::Ident(name) if name == "err") {
                    Kind::Err(boxed(&arguments[0].value)?)
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
                step_node(
                    step,
                    boxed(input)?,
                    self.arguments(arguments)?,
                    written.span.clone(),
                    self.lifted.contains(&written.span),
                    self.declarations.get(&written.span).copied(),
                )
            }
        };
        Ok(Expr { kind, ty, span })
    }
}

/// The values of a step's or a constructor's arguments.
fn argument_values(arguments: &[Arg]) -> Vec<&Expr> {
    arguments.iter().map(|argument| &argument.value).collect()
}

/// The expressions an iterating function's body holds, in order.
fn block_expressions<'e>(statements: &'e [BlockStmt], held: &mut Vec<&'e Expr>) {
    for statement in statements {
        match statement {
            BlockStmt::Bind { value, .. } | BlockStmt::Yield(value) => held.push(value),
            BlockStmt::For { iterable, body, .. } => {
                held.push(iterable);
                block_expressions(body, held);
            }
        }
    }
}

impl Program {
    /// The roots of the program's expression trees: each statement's, a
    /// function's body and an implementation's among them, then each
    /// imported module's.
    pub(crate) fn roots(&self) -> Vec<&Expr> {
        let mut roots = Vec::new();
        for statement in &self.statements {
            match statement {
                Stmt::Bind { value, .. } | Stmt::Expr(value) => roots.push(value),
                Stmt::Fn { body, .. } => roots.push(body.as_ref()),
                Stmt::Impl { methods, .. } => {
                    roots.extend(methods.iter().map(|(_, _, body, _)| body.as_ref()));
                }
                Stmt::Contract { rules, .. } => {
                    roots.extend(rules.iter().map(|rule| rule.body.as_ref()));
                }
                Stmt::Unit => {}
            }
        }
        for module in &self.modules {
            roots.extend(module.program.roots());
        }
        roots
    }

    /// Every pipeline of the program, each by the step node that ends it.
    pub(crate) fn pipelines(&self) -> Vec<&Expr> {
        let mut found = Vec::new();
        for root in self.roots() {
            root.pipelines(&mut found);
        }
        found
    }
}

/// A host operation's step node, carrying the operation type the planner
/// reads.
fn step_node(
    step: &'static Operation,
    input: Box<Expr>,
    arguments: Vec<Arg>,
    at: Range<usize>,
    lifted: bool,
    declaration: Option<crate::extensions::binding::DeclarationId>,
) -> Kind {
    let plan = StepPlan::new(OperationType::host(step, &arguments));
    Kind::Step {
        step,
        input,
        arguments,
        at,
        lifted,
        declaration,
        plan,
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
    fn a_call_a_receiver_call_and_a_pipeline_step_resolve_one_declaration() {
        fn declarations(expression: &Expr, found: &mut Vec<String>) {
            if let Kind::Step {
                input, declaration, ..
            } = &expression.kind
            {
                let environment = crate::extensions::binding::Environment::get().unwrap();
                found.push(declaration.map_or_else(String::new, |id| environment.qualified(id)));
                declarations(input, found);
            }
        }
        let vault = crate::vault::Vault::empty();
        let mut found = Vec::new();
        for source in [
            "\"x\".pad_left(3, \"0\")",
            "string.pad_left(\"x\", 3, \"0\")",
            "\"x\" | pad_left(3, \"0\")",
            "pad_left(\"x\", 3, \"0\")",
        ] {
            let syntax = crate::parser::parse(source).unwrap();
            let program = crate::checker::check(&syntax, &vault).unwrap();
            let [statement] = program.statements.as_slice() else {
                panic!("one statement");
            };
            let Stmt::Expr(expression) = statement else {
                panic!("an expression");
            };
            declarations(expression, &mut found);
        }
        assert_eq!(found, vec!["string.pad_left"; 4]);
        // An overloaded name resolves the form the call took.
        let syntax = crate::parser::parse("[2, 1] | sort").unwrap();
        let program = crate::checker::check(&syntax, &vault).unwrap();
        let Stmt::Expr(expression) = &program.statements[0] else {
            panic!("an expression");
        };
        let Kind::Step { declaration, .. } = &expression.kind else {
            panic!("a step");
        };
        let environment = crate::extensions::binding::Environment::get().unwrap();
        let sort = environment.of(crate::extensions::CORE.step("sort").unwrap());
        assert_eq!(*declaration, Some(sort[0]));
    }

    #[test]
    fn checked_execution_can_be_shared_with_a_future_planner() {
        fn shared<T: Send + Sync>() {}
        shared::<Program>();
    }
}
