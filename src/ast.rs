//! Private syntax tree shared by parsing and checking, before execution lowering.

use std::ops::Range;

/// A whole program: statements, whose last expression is the result.
#[derive(Debug)]
pub(crate) struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug)]
pub(crate) enum Stmt {
    /// `name = expr` or `name : Type = expr`.
    Bind {
        name: String,
        annotation: Option<TypeAnn>,
        value: Expr,
    },
    /// `import <extension>` — bind an extension's steps into the program.
    Import {
        name: String,
        span: Range<usize>,
    },
    /// `type Name<P> : Parent { field : Type }`, or `type Name = union { A, B }`.
    Type(TypeDecl),
    /// `fn name(p: T, ..) : R = body`.
    Fn(FnDecl),
    Expr(Expr),
}

/// A declared function: a name, typed parameters, a result type and a body.
#[derive(Debug)]
pub(crate) struct FnDecl {
    pub name: String,
    /// Generic parameters, `T` in `fn first<T>(…)`.
    pub generics: Vec<String>,
    pub parameters: Vec<Param>,
    pub result: TypeAnn,
    pub body: Expr,
    pub span: Range<usize>,
}

/// One statement of a function's block body.
#[derive(Debug)]
pub(crate) enum BlockStmt {
    /// `name = expr`, visible to the statements after it.
    Bind { name: String, value: Expr },
    /// `yield expr`: one value of the traversal the function produces.
    Yield(Expr),
    /// `for name in iterable { ... }`: the body once per value, in order.
    For {
        name: String,
        iterable: Expr,
        body: Vec<BlockStmt>,
    },
}

/// One parameter of a declared function.
#[derive(Debug)]
pub(crate) struct Param {
    pub name: String,
    pub annotation: TypeAnn,
    /// `xs: ...T`: the trailing arguments of a call, collected.
    pub variadic: bool,
}

/// A declared type: what it is called, what it narrows, and what it holds.
#[derive(Debug)]
pub(crate) struct TypeDecl {
    pub name: String,
    pub abstract_type: bool,
    pub bounds: Vec<(String, TypeAnn)>,
    /// The generic parameters, `S` and `T` in `type Edge<S, T>`.
    pub parameters: Vec<String>,
    /// What it narrows: one type, or a supertype set written `{A, B}`.
    pub supertypes: Vec<TypeAnn>,
    /// The record body, empty when the declaration has none.
    pub fields: Vec<Field>,
    /// The alternatives written `= union {A, B}`. A union lists what it is
    /// rather than what it narrows, so the parser leaves `supertypes` and
    /// `fields` empty whenever this is present.
    pub union: Option<Vec<TypeAnn>>,
    pub span: Range<usize>,
}

/// One entry in a record body.
#[derive(Debug)]
pub(crate) struct Field {
    pub name: String,
    /// Whether the field is written `name? : Type`.
    pub optional: bool,
    pub annotation: TypeAnn,
    pub span: Range<usize>,
}

/// A written type, such as `Card` or `Set<Card>`.
#[derive(Debug, Clone)]
pub(crate) struct TypeAnn {
    pub name: String,
    pub arguments: Vec<TypeAnn>,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub(crate) struct Expr {
    pub kind: Kind,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub(crate) struct Arg<E = Expr> {
    /// `Some` for a named argument, `depth = 1`.
    pub name: Option<String>,
    pub value: E,
}

#[derive(Debug)]
pub(crate) enum Kind {
    /// A materialized sequence literal.
    List(Vec<Expr>),
    /// An unordered collection literal.
    Set(Vec<Expr>),
    /// A collection constructor with explicit type arguments.
    Construct {
        annotation: TypeAnn,
        arguments: Vec<Arg>,
    },
    /// The block body of an iterating function. Written only as a `fn` body.
    Block(Vec<BlockStmt>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// A bound name. `cards` is one, seeded from the vault.
    Ident(String),
    /// `[[name]]` — a reference to a document in the current namespace.
    DocRef(String),
    /// `{ key: value, .. }` — an open tree.
    Data(Vec<(String, Expr)>),
    /// `a + b`, `a - b`, `a * b`, `a / b` or `a % b`.
    Arith(Arith, Box<Expr>, Box<Expr>),
    /// `a == b` or `a != b`.
    Compare {
        left: Box<Expr>,
        right: Box<Expr>,
        negated: bool,
    },
    /// `a < b`, `a <= b`, `a > b` or `a >= b`.
    Relate {
        left: Box<Expr>,
        right: Box<Expr>,
        relation: Relation,
    },
    /// `a and b` or `a or b`.
    Logic(Logic, Box<Expr>, Box<Expr>),
    /// `not a`.
    Not(Box<Expr>),
    /// `input | step`, where the step is a name or a call.
    Pipe(Box<Expr>, Box<Expr>),
    /// `[[a]] -> [[b]] { .. }` — the link operator.
    Link {
        source: Box<Expr>,
        target: Box<Expr>,
        data: Option<Box<Expr>>,
    },
    Field(Box<Expr>, String),
    Call {
        callee: Box<Expr>,
        arguments: Vec<Arg>,
    },
    /// `p => p.title`, or `(p: Card) => p.title` with its parameters typed.
    Lambda {
        parameters: Vec<String>,
        annotations: Vec<Option<TypeAnn>>,
        body: Box<Expr>,
    },
    /// `match subject { pattern => result, .. }`.
    Match {
        subject: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// `tee name = branch` as a pipeline step: `branch` is a pipeline over
    /// [`TEE_INPUT`], the value arriving at the step.
    Tee {
        name: String,
        branch: Box<Expr>,
    },
    /// `value given condition`, and `value given condition otherwise other`.
    Given {
        value: Box<Expr>,
        condition: Box<Expr>,
        otherwise: Option<Box<Expr>>,
    },
}

/// The name a `tee` branch reads its input by. It cannot be written in a
/// program, so no binding can shadow it.
pub(crate) const TEE_INPUT: &str = "%tee";

/// One case of a `match`: the first arm whose pattern fits, and whose guard
/// holds, produces the result.
#[derive(Debug)]
pub(crate) struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Range<usize>,
}

/// What a `match` arm fits.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Pattern {
    /// `_`: anything, binding nothing.
    Wildcard,
    /// A name: anything, bound to that name in the arm.
    Bind(String),
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    /// `some(p)`: a present option whose value fits `p`.
    Some(Box<Pattern>),
    /// `none`: an absent option.
    None,
    /// `U name` or `U _`: a value that is a `U`, bound at that type.
    Type(String, Box<Pattern>),
}

impl Pattern {
    /// Whether the pattern fits every value of the type it is checked at.
    pub fn is_irrefutable(&self) -> bool {
        matches!(self, Self::Wildcard | Self::Bind(_))
    }
}

/// An arithmetic operator. Each is homogeneous: two `Int`s or two `Float`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl Arith {
    /// What the checker calls the operation in a diagnostic.
    pub fn noun(self) -> &'static str {
        match self {
            Self::Add => "addition",
            Self::Sub => "subtraction",
            Self::Mul => "multiplication",
            Self::Div => "division",
            Self::Rem => "remainder",
        }
    }
}

/// An ordering relation between two values of one `Orderable` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relation {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl Relation {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
        }
    }

    /// Whether the relation holds of an ordering between its operands.
    pub fn holds(self, ordering: std::cmp::Ordering) -> bool {
        match self {
            Self::Less => ordering.is_lt(),
            Self::LessEqual => ordering.is_le(),
            Self::Greater => ordering.is_gt(),
            Self::GreaterEqual => ordering.is_ge(),
        }
    }
}

/// A boolean connective. Both short-circuit, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Logic {
    And,
    Or,
}

impl Logic {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}
