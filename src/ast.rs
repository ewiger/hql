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
        /// Where the name is written.
        name_span: Range<usize>,
        annotation: Option<TypeAnn>,
        value: Expr,
    },
    /// `import a.b [as c]` — make one module visible under a local name:
    /// its last segment, or the alias.
    Import {
        path: Vec<String>,
        alias: Option<String>,
        span: Range<usize>,
    },
    /// `module for T` — the receiver pattern a module file opens with.
    ModuleFor {
        pattern: TypeAnn,
        span: Range<usize>,
    },
    /// `type Name<P> : Parent { field : Type }`, or `type Name = union { A, B }`.
    Type(TypeDecl),
    /// `fn name(p: T, ..) : R = body`.
    Fn(FnDecl),
    /// `name(p, ..) := body`: a function whose types the checker writes.
    Alias(AliasDecl),
    /// `trait Name { fn m(self, ..) : R [= body] .. }`: a capability contract.
    Trait(TraitDecl),
    /// `impl Name for Type { fn m(self, ..) : R = body .. }`: a type's
    /// implementation of a trait.
    Impl(ImplDecl),
    Expr(Expr),
}

/// A declared trait: a name and the functions a type implements to satisfy
/// it, some with a default body.
#[derive(Debug)]
pub(crate) struct TraitDecl {
    pub name: String,
    pub methods: Vec<MethodDecl>,
    pub span: Range<usize>,
}

/// An implementation of a trait for a type.
#[derive(Debug)]
pub(crate) struct ImplDecl {
    pub trait_name: String,
    pub target: TypeAnn,
    pub methods: Vec<MethodDecl>,
    pub span: Range<usize>,
}

/// One function of a trait or an implementation: `self` first, then the
/// typed parameters, the result, and a body unless the trait leaves it to
/// the implementation. `Self` in a signature is the implementing type.
#[derive(Debug)]
pub(crate) struct MethodDecl {
    pub name: String,
    /// Whether the function takes `self`, and so is reached as `value.f()`.
    pub has_self: bool,
    /// The parameters after `self`.
    pub parameters: Vec<Param>,
    pub result: TypeAnn,
    pub body: Option<Expr>,
    pub span: Range<usize>,
}

/// A declared function: a name, typed parameters, a result type and a body.
#[derive(Debug)]
pub(crate) struct FnDecl {
    pub name: String,
    /// Whether the first parameter is `self`: a receiver function of the
    /// module the declaration is in, reached as `value.name(…)`.
    pub has_self: bool,
    /// Generic parameters, `T` in `fn first<T>(…)`.
    pub generics: Vec<String>,
    /// The bound a generic parameter is written with, `T : Orderable`.
    pub bounds: Vec<(String, TypeAnn)>,
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

/// `name(p, ..) := body`. With a fragment body the input is implicit and
/// named [`ALIAS_INPUT`]; with an expression body the first parameter is the
/// input and is annotated.
#[derive(Debug)]
pub(crate) struct AliasDecl {
    pub name: String,
    /// The input first, then the written parameters.
    pub parameters: Vec<AliasParam>,
    pub body: Expr,
    pub span: Range<usize>,
}

/// One parameter of an alias: its type when written.
#[derive(Debug)]
pub(crate) struct AliasParam {
    pub name: String,
    pub annotation: Option<TypeAnn>,
    pub span: Range<usize>,
}

/// The combiners a host provides, by name, and whether each is commutative:
/// an order-independent fold over one may visit any collection.
pub(crate) const COMBINERS: [(&str, bool); 5] = [
    ("min", true),
    ("max", true),
    ("union", true),
    ("merge", false),
    ("concat", false),
];

/// The name a fragment alias's implicit input takes. It cannot be written.
pub(crate) const ALIAS_INPUT: &str = "%input";

/// One parameter of a declared function.
#[derive(Debug)]
pub(crate) struct Param {
    pub name: String,
    /// Where the name is written.
    pub span: Range<usize>,
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
    /// `@[[name]]` — a reference to a document in the current namespace.
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
    /// `@[[a]] -> @[[b]] { .. }` — the link operator.
    Link {
        source: Box<Expr>,
        target: Box<Expr>,
        data: Option<Box<Expr>>,
    },
    Field(Box<Expr>, String),
    /// `xs[i]` — the element at a position of a sequence, or of a tree.
    Index(Box<Expr>, Box<Expr>),
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
    /// `ok(p)`: a result that succeeded with a value fitting `p`.
    Ok(Box<Pattern>),
    /// `err(p)`: a result that failed with an error fitting `p`.
    Err(Box<Pattern>),
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
