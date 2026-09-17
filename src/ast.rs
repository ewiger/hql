//! Private syntax tree shared by parsing, type checking, and evaluation.

use std::ops::Range;

#[derive(Debug)]
pub(crate) struct Expr {
    pub kind: Kind,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub(crate) enum Kind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Add(Box<Expr>, Box<Expr>),
}
