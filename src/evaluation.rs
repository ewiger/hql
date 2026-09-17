//! Pure evaluation with checked arithmetic and no document or filesystem access.

use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;
use crate::values::Value;

pub(crate) fn evaluate(expression: &Expr) -> Result<Value, Diagnostic> {
    match &expression.kind {
        Kind::Int(value) => Ok(Value::Int(*value)),
        Kind::Float(value) => Ok(Value::Float(*value)),
        Kind::Bool(value) => Ok(Value::Bool(*value)),
        Kind::Add(left, right) => {
            let overflow = || Diagnostic::Overflow {
                span: expression.span.clone(),
            };
            match (evaluate(left)?, evaluate(right)?) {
                (Value::Int(a), Value::Int(b)) => {
                    a.checked_add(b).map(Value::Int).ok_or_else(overflow)
                }
                // Values stay finite, so a saturating sum is reported, not returned.
                (Value::Float(a), Value::Float(b)) => {
                    let sum = a + b;
                    sum.is_finite()
                        .then_some(Value::Float(sum))
                        .ok_or_else(overflow)
                }
                (a, _) => Err(Diagnostic::Type {
                    span: if matches!(a, Value::Int(_) | Value::Float(_)) {
                        right.span.clone()
                    } else {
                        left.span.clone()
                    },
                }),
            }
        }
    }
}
