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
    Expr(Expr),
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
        parameter: String,
        body: Box<Expr>,
    },
}
