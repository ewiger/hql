//! Pure evaluation with checked arithmetic and no document or filesystem access.

use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;
use crate::values::Value;

pub(crate) fn evaluate(expression: &Expr) -> Result<Value, Diagnostic> {
    match &expression.kind {
        Kind::Int(value) => Ok(Value::Int(*value)),
        Kind::Bool(value) => Ok(Value::Bool(*value)),
        Kind::Add(left, right) => {
            let Value::Int(a) = evaluate(left)? else {
                return Err(Diagnostic::Type {
                    span: left.span.clone(),
                });
            };
            let Value::Int(b) = evaluate(right)? else {
                return Err(Diagnostic::Type {
                    span: right.span.clone(),
                });
            };
            a.checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| Diagnostic::Overflow {
                    span: expression.span.clone(),
                })
        }
    }
}
