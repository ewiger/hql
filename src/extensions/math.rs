//! The `math` module: numeric functions over `Int` and `Float`, and the
//! conversions between the two.

use super::declared;
use super::{EvalCx, Operation};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::Value;
use std::ops::Range;

/// Every step the `math` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("abs", "The number without its sign.", eval_abs),
    declared(
        "pow",
        "The number raised to an exponent of the same type.",
        eval_pow,
    ),
    declared("sign", "-1, 0 or 1 by the sign of the number.", eval_sign),
    declared("float", "The Int as a Float.", eval_float),
    declared(
        "int",
        "The Float as an Int, its fraction dropped.",
        eval_int,
    ),
    declared("gcd", "The greatest common divisor of two Ints.", eval_gcd),
    declared("sqrt", "The square root.", eval_sqrt),
    declared(
        "floor",
        "The greatest whole number not above the value.",
        eval_floor,
    ),
    declared(
        "ceil",
        "The least whole number not below the value.",
        eval_ceil,
    ),
    declared(
        "round",
        "The nearest whole number, halves away from zero.",
        eval_round,
    ),
    declared("exp", "e raised to the value.", eval_exp),
    declared("ln", "The natural logarithm.", eval_ln),
    declared("log10", "The base-10 logarithm.", eval_log10),
    declared("sin", "The sine of an angle in radians.", eval_sin),
    declared("cos", "The cosine of an angle in radians.", eval_cos),
    declared("tan", "The tangent of an angle in radians.", eval_tan),
];

fn finite(value: f64, span: &Range<usize>) -> Result<Value, Diagnostic> {
    if value.is_finite() {
        Ok(Value::Float(value))
    } else {
        Err(Diagnostic::Overflow { span: span.clone() })
    }
}

fn number(input: &Value, name: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
    match input {
        Value::Int(_) | Value::Float(_) => Ok(input.clone()),
        Value::Present(inner) => number(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a number, not {}", other.type_of()),
        )),
    }
}

fn float_of(input: &Value, name: &str, span: &Range<usize>) -> Result<f64, Diagnostic> {
    match number(input, name, span)? {
        Value::Float(x) => Ok(x),
        Value::Int(n) => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a Float, and {n} is an Int"),
        )),
        _ => unreachable!("number() answers a number"),
    }
}

macro_rules! float_unary {
    ($($eval:ident => $name:literal, $op:expr;)*) => {
        $(fn $eval(
            _: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value, Diagnostic> {
            let x = float_of(&input, $name, &span)?;
            let op: fn(f64) -> f64 = $op;
            finite(op(x), &span)
        })*
    };
}

float_unary! {
    eval_sqrt => "sqrt", f64::sqrt;
    eval_floor => "floor", f64::floor;
    eval_ceil => "ceil", f64::ceil;
    eval_round => "round", f64::round;
    eval_exp => "exp", f64::exp;
    eval_ln => "ln", f64::ln;
    eval_log10 => "log10", f64::log10;
    eval_sin => "sin", f64::sin;
    eval_cos => "cos", f64::cos;
    eval_tan => "tan", f64::tan;
}

fn eval_abs(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match number(&input, "abs", &span)? {
        Value::Int(n) => n
            .checked_abs()
            .map(Value::Int)
            .ok_or(Diagnostic::Overflow { span }),
        Value::Float(x) => Ok(Value::Float(x.abs())),
        _ => unreachable!("number() answers a number"),
    }
}

fn eval_pow(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let base = number(&input, "pow", &span)?;
    let exponent = arguments
        .first()
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`pow` needs an exponent"))?;
    let exponent = cx.evaluate(&exponent.value)?;
    match (base, exponent) {
        (Value::Int(base), Value::Int(exponent)) => {
            if exponent < 0 {
                return Err(Diagnostic::runtime(
                    span,
                    "`pow` over Ints takes an exponent of 0 or more",
                ));
            }
            u32::try_from(exponent)
                .ok()
                .and_then(|exponent| base.checked_pow(exponent))
                .map(Value::Int)
                .ok_or(Diagnostic::Overflow { span })
        }
        (Value::Float(base), Value::Float(exponent)) => finite(base.powf(exponent), &span),
        (base, exponent) => Err(Diagnostic::runtime(
            span,
            format!(
                "`pow` raises a number to an exponent of its own type, not {} to {}",
                base.type_of(),
                exponent.type_of()
            ),
        )),
    }
}

fn eval_sign(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Int(match number(&input, "sign", &span)? {
        Value::Int(n) => n.signum(),
        Value::Float(x) => {
            if x == 0.0 {
                0
            } else if x.is_sign_negative() {
                -1
            } else {
                1
            }
        }
        _ => unreachable!("number() answers a number"),
    }))
}

fn eval_float(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match number(&input, "float", &span)? {
        // Precision past 2^53 is lost, as it is in every language with
        // doubles; the conversion itself cannot fail.
        #[allow(clippy::cast_precision_loss)]
        Value::Int(n) => Ok(Value::Float(n as f64)),
        other => Ok(other),
    }
}

fn eval_int(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match number(&input, "int", &span)? {
        Value::Float(x) => {
            let truncated = x.trunc();
            // The Int range is exactly what fits; anything beyond overflows.
            #[allow(clippy::cast_precision_loss)]
            if truncated.is_finite() && truncated >= i64::MIN as f64 && truncated < i64::MAX as f64
            {
                #[allow(clippy::cast_possible_truncation)]
                Ok(Value::Int(truncated as i64))
            } else {
                Err(Diagnostic::Overflow { span })
            }
        }
        other => Ok(other),
    }
}

fn eval_gcd(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Int(mut a) = number(&input, "gcd", &span)? else {
        return Err(Diagnostic::runtime(span, "`gcd` needs two Ints"));
    };
    let other = arguments
        .first()
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`gcd` needs two Ints"))?;
    let Value::Int(mut b) = cx.evaluate(&other.value)? else {
        return Err(Diagnostic::runtime(span, "`gcd` needs two Ints"));
    };
    a = a
        .checked_abs()
        .ok_or(Diagnostic::Overflow { span: span.clone() })?;
    b = b.checked_abs().ok_or(Diagnostic::Overflow { span })?;
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Ok(Value::Int(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec()
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }
}
