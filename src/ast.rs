//! Private syntax tree shared by parsing, type checking, and evaluation.

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
    /// `type Name<P> : Parent { field : Type }`.
    Type(TypeDecl),
    Expr(Expr),
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
pub(crate) struct Arg {
    /// `Some` for a named argument, `depth = 1`.
    pub name: Option<String>,
    pub value: Expr,
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
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// A bound name.
    Ident(String),
    /// The `cards` keyword: the current vault's cards.
    Cards,
    /// `[[name]]` — a reference to a document in the current namespace.
    DocRef(String),
    /// `{ key: value, .. }` — an open tree.
    Data(Vec<(String, Expr)>),
    Add(Box<Expr>, Box<Expr>),
    /// `a == b` or `a != b`.
    Compare {
        left: Box<Expr>,
        right: Box<Expr>,
        negated: bool,
    },
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
    /// `p => p.title`.
    Lambda {
        parameters: Vec<String>,
        body: Box<Expr>,
    },
}
