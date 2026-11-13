//! The `stats` module (HQL-0041 part 2): the natives behind every
//! declaration of `lib/std/stats/mod.hql`.
//!
//! The reductions fold a partial state that two partitions combine. Over
//! `Float` values the state is `(n, mean, M2)`, Welford's running mean and
//! sum of squared deviations, and two states combine by Chan's pairwise
//! update. Over `Int` and `Decimal` values the state is `(n, Σx, Σx²)`, held
//! exact, and two states combine by addition: Chan's update in exact
//! arithmetic, so a split never changes the result. A statistic is then one
//! division of exact values, rounded once.
//!
//! `localhost` folds an input as one partition; [`Moments::combine`] and
//! [`Sums::combine`] are what a backend that splits the fold applies, and
//! the tests hold a split fold equal to the whole one.

use super::collections::{element_type, elements, in_element_order, wrong};
use super::{EvalCx, Operation, Purity, Shape, Signature, declared};
use crate::decimal::Decimal;
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeRef, Value};
use bigdecimal::num_bigint::BigInt;
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

/// A statistic whose partial states combine, so the executor may split it.
const fn reduction(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Pure,
        shape: Shape::Reduction,
        check: None,
        eval,
    }
}

/// Every step the `stats` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    reduction("mean", eval_mean),
    reduction("variance", eval_variance),
    reduction("stdev", eval_stdev),
    reduction("pvariance", eval_pvariance),
    reduction("pstdev", eval_pstdev),
    reduction("covariance", eval_covariance),
    reduction("correlation", eval_correlation),
    declared("median", eval_median),
    declared("quantile", eval_quantile),
    declared("quantiles", eval_quantiles),
    declared("mode", eval_mode),
];

type Result<T> = std::result::Result<T, Diagnostic>;

fn fail(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message)
}

/// The values of a numeric collection, at the type its elements have.
enum Numbers {
    Int(Vec<i64>),
    Float(Vec<f64>),
    Decimal(Vec<Decimal>),
}

/// Which of the three number types a collection holds: its element type,
/// or its values' where the element type does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Float,
    Decimal,
}

impl Kind {
    fn of_type(ty: &TypeRef) -> Option<Self> {
        if *ty == TypeRef::DECIMAL {
            Some(Self::Decimal)
        } else if *ty == TypeRef::FLOAT {
            Some(Self::Float)
        } else if *ty == TypeRef::INT {
            Some(Self::Int)
        } else {
            None
        }
    }

    fn of_value(value: &Value) -> Option<Self> {
        match value {
            Value::Int(_) => Some(Self::Int),
            Value::Float(_) => Some(Self::Float),
            Value::Decimal(_) => Some(Self::Decimal),
            Value::Present(inner) => Self::of_value(inner),
            _ => None,
        }
    }

    /// The type of a statistic of this kind: a `Decimal` of `Decimal`s, a
    /// `Float` otherwise.
    fn result(self) -> TypeRef {
        match self {
            Self::Decimal => TypeRef::DECIMAL,
            Self::Int | Self::Float => TypeRef::FLOAT,
        }
    }
}

fn int_of(value: &Value, name: &str, span: &Range<usize>) -> Result<i64> {
    match value {
        Value::Int(n) => Ok(*n),
        Value::Present(inner) => int_of(inner, name, span),
        _ => Err(wrong(name, "Int occurrences", span)),
    }
}

fn float_of(value: &Value, name: &str, span: &Range<usize>) -> Result<f64> {
    match value {
        Value::Float(x) => Ok(*x),
        #[allow(clippy::cast_precision_loss)]
        Value::Int(n) => Ok(*n as f64),
        Value::Present(inner) => float_of(inner, name, span),
        _ => Err(wrong(name, "Float occurrences", span)),
    }
}

fn decimal_of(value: &Value, name: &str, span: &Range<usize>) -> Result<Decimal> {
    match value {
        Value::Decimal(x) => Ok(x.clone()),
        Value::Int(n) => Ok(Decimal::from_int(*n)),
        Value::Present(inner) => decimal_of(inner, name, span),
        _ => Err(wrong(name, "Decimal occurrences", span)),
    }
}

/// The values of a collection of numbers, with their kind.
fn numbers(input: &Value, name: &str, span: &Range<usize>) -> Result<(Kind, Numbers)> {
    let held = elements(input, name, span)?;
    let kind = Kind::of_type(&element_type(input))
        .or_else(|| held.first().and_then(Kind::of_value))
        .unwrap_or(Kind::Int);
    let numbers = match kind {
        Kind::Int => Numbers::Int(
            held.iter()
                .map(|value| int_of(value, name, span))
                .collect::<Result<_>>()?,
        ),
        Kind::Float => Numbers::Float(
            held.iter()
                .map(|value| float_of(value, name, span))
                .collect::<Result<_>>()?,
        ),
        Kind::Decimal => Numbers::Decimal(
            held.iter()
                .map(|value| decimal_of(value, name, span))
                .collect::<Result<_>>()?,
        ),
    };
    Ok((kind, numbers))
}

/// The `first` and `second` fields of a pair.
fn halves<'a>(value: &'a Value, name: &str, span: &Range<usize>) -> Result<(&'a Value, &'a Value)> {
    let Value::Record(record) = value else {
        return Err(wrong(name, "Pair occurrences", span));
    };
    let field = |wanted: &str| {
        record
            .fields
            .iter()
            .find(|(field, _)| field == wanted)
            .map(|(_, value)| value)
            .ok_or_else(|| wrong(name, "Pair occurrences", span))
    };
    Ok((field("first")?, field("second")?))
}

/// The pairs of a collection of `Pair`s of numbers, with their kind.
fn pairs(input: &Value, name: &str, span: &Range<usize>) -> Result<(Kind, PairNumbers)> {
    let held = elements(input, name, span)?;
    let element = element_type(input);
    let kind = element
        .args
        .first()
        .and_then(Kind::of_type)
        .or_else(|| {
            held.first()
                .and_then(|pair| halves(pair, name, span).ok())
                .and_then(|(first, _)| Kind::of_value(first))
        })
        .unwrap_or(Kind::Int);
    let mut halved = Vec::with_capacity(held.len());
    for pair in &held {
        halved.push(halves(pair, name, span)?);
    }
    Ok((
        kind,
        match kind {
            Kind::Float => PairNumbers::Float(
                halved
                    .into_iter()
                    .map(|(x, y)| Ok((float_of(x, name, span)?, float_of(y, name, span)?)))
                    .collect::<Result<_>>()?,
            ),
            Kind::Int | Kind::Decimal => PairNumbers::Exact(
                halved
                    .into_iter()
                    .map(|(x, y)| Ok((decimal_of(x, name, span)?, decimal_of(y, name, span)?)))
                    .collect::<Result<_>>()?,
            ),
        },
    ))
}

/// The pairs of a covariance or a correlation: `Float`s, or exact values.
enum PairNumbers {
    Float(Vec<(f64, f64)>),
    Exact(Vec<(Decimal, Decimal)>),
}

// The partial states.

/// The state of a `Float` reduction: how many values, their running mean,
/// and the sum of squared deviations from it (Welford's).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Moments {
    n: u64,
    mean: f64,
    m2: f64,
}

impl Moments {
    /// The state with one more value.
    pub(crate) fn add(self, x: f64) -> Self {
        let n = self.n + 1;
        let delta = x - self.mean;
        #[allow(clippy::cast_precision_loss)]
        let mean = self.mean + delta / n as f64;
        Self {
            n,
            mean,
            m2: self.m2 + delta * (x - mean),
        }
    }

    /// The state of two partitions of an input (Chan's pairwise update).
    // What a backend that splits the fold applies; `localhost` folds one
    // partition, and the tests hold a split fold equal to the whole.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn combine(self, other: Self) -> Self {
        if self.n == 0 {
            return other;
        }
        if other.n == 0 {
            return self;
        }
        let n = self.n + other.n;
        #[allow(clippy::cast_precision_loss)]
        let (a, b, whole) = (self.n as f64, other.n as f64, n as f64);
        let delta = other.mean - self.mean;
        Self {
            n,
            mean: self.mean + delta * b / whole,
            m2: self.m2 + other.m2 + delta * delta * a * b / whole,
        }
    }

    fn of(values: &[f64]) -> Self {
        values.iter().copied().fold(Self::default(), Self::add)
    }
}

/// The state of a `Float` covariance or correlation: the moments of each
/// side and the sum of the products of their deviations.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CoMoments {
    x: Moments,
    y: Moments,
    c: f64,
}

impl CoMoments {
    /// The state with one more pair.
    pub(crate) fn add(self, (x, y): (f64, f64)) -> Self {
        let dx = x - self.x.mean;
        let xs = self.x.add(x);
        let ys = self.y.add(y);
        Self {
            x: xs,
            y: ys,
            c: self.c + dx * (y - ys.mean),
        }
    }

    /// The state of two partitions of an input.
    // What a backend that splits the fold applies; `localhost` folds one
    // partition, and the tests hold a split fold equal to the whole.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn combine(self, other: Self) -> Self {
        if self.x.n == 0 {
            return other;
        }
        if other.x.n == 0 {
            return self;
        }
        #[allow(clippy::cast_precision_loss)]
        let (a, b) = (self.x.n as f64, other.x.n as f64);
        let dx = other.x.mean - self.x.mean;
        let dy = other.y.mean - self.y.mean;
        Self {
            x: self.x.combine(other.x),
            y: self.y.combine(other.y),
            c: self.c + other.c + dx * dy * a * b / (a + b),
        }
    }

    fn of(values: &[(f64, f64)]) -> Self {
        values.iter().copied().fold(Self::default(), Self::add)
    }
}

/// The exact state of an `Int` or `Decimal` reduction: how many values,
/// their sum and the sum of their squares; over pairs, each side's and the
/// sum of the products. Two states combine by addition.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sums {
    n: i64,
    x: Decimal,
    xx: Decimal,
    y: Decimal,
    yy: Decimal,
    xy: Decimal,
}

impl Default for Sums {
    fn default() -> Self {
        let zero = Decimal::from_int(0);
        Self {
            n: 0,
            x: zero.clone(),
            xx: zero.clone(),
            y: zero.clone(),
            yy: zero.clone(),
            xy: zero,
        }
    }
}

type Exact<T> = std::result::Result<T, crate::decimal::Refusal>;

impl Sums {
    /// The state with one more value.
    pub(crate) fn add(self, x: &Decimal) -> Exact<Self> {
        Ok(Self {
            n: self.n + 1,
            xx: self.xx.add(&x.mul(x)?)?,
            x: self.x.add(x)?,
            ..self
        })
    }

    /// The state with one more pair.
    pub(crate) fn add_pair(self, (x, y): &(Decimal, Decimal)) -> Exact<Self> {
        Ok(Self {
            n: self.n + 1,
            xx: self.xx.add(&x.mul(x)?)?,
            yy: self.yy.add(&y.mul(y)?)?,
            xy: self.xy.add(&x.mul(y)?)?,
            x: self.x.add(x)?,
            y: self.y.add(y)?,
        })
    }

    /// The state of two partitions of an input.
    // What a backend that splits the fold applies; `localhost` folds one
    // partition, and the tests hold a split fold equal to the whole.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn combine(self, other: &Self) -> Exact<Self> {
        Ok(Self {
            n: self.n + other.n,
            x: self.x.add(&other.x)?,
            xx: self.xx.add(&other.xx)?,
            y: self.y.add(&other.y)?,
            yy: self.yy.add(&other.yy)?,
            xy: self.xy.add(&other.xy)?,
        })
    }

    fn of(values: &[Decimal]) -> Exact<Self> {
        values.iter().try_fold(Self::default(), Self::add)
    }

    fn of_pairs(values: &[(Decimal, Decimal)]) -> Exact<Self> {
        values.iter().try_fold(Self::default(), Self::add_pair)
    }

    fn count(&self) -> Decimal {
        Decimal::from_int(self.n)
    }

    /// `n Σx² − (Σx)²`: `n²` times the population variance.
    fn spread_x(&self) -> Exact<Decimal> {
        self.count().mul(&self.xx)?.sub(&self.x.mul(&self.x)?)
    }

    /// `n Σy² − (Σy)²`.
    fn spread_y(&self) -> Exact<Decimal> {
        self.count().mul(&self.yy)?.sub(&self.y.mul(&self.y)?)
    }

    /// `n Σxy − Σx Σy`: `n²` times the population covariance.
    fn spread_xy(&self) -> Exact<Decimal> {
        self.count().mul(&self.xy)?.sub(&self.x.mul(&self.y)?)
    }

    /// `n (n − 1)`, the divisor of a sample statistic of `n Σ…` terms.
    fn sample_divisor(&self) -> Decimal {
        Decimal::from_int(self.n)
            .mul(&Decimal::from_int(self.n - 1))
            .unwrap_or_else(|_| {
                // Unreachable for an `i64` count: two `i64`s fit 512 bits.
                Decimal::from_int(1)
            })
    }

    /// `n²`, the divisor of a population statistic.
    fn population_divisor(&self) -> Decimal {
        self.count()
            .mul(&self.count())
            .unwrap_or_else(|_| Decimal::from_int(1))
    }
}

// Exact square roots.

/// A `Decimal`'s significand and scale: `value = significand × 10^-scale`.
fn parts(value: &Decimal) -> (BigInt, i64) {
    let text = value.to_string();
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |rest| (true, rest));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let significand: BigInt = format!("{whole}{fraction}").parse().unwrap_or_default();
    let scale = i64::try_from(fraction.len()).unwrap_or(0);
    (if negative { -significand } else { significand }, scale)
}

/// How many decimal digits a non-negative integer has.
fn digits(value: &BigInt) -> i64 {
    i64::try_from(value.to_string().trim_start_matches('-').len()).unwrap_or(0)
}

fn ten_to(power: i64) -> BigInt {
    BigInt::from(10).pow(u32::try_from(power).unwrap_or(0))
}

/// `sqrt(numerator / denominator)` for a non-negative ratio, rounded half to
/// even to [`crate::decimal::DIVISION_DIGITS`] significant digits, and exact
/// where the root terminates: rounded once, from the exact ratio.
fn sqrt_ratio(numerator: &Decimal, denominator: &Decimal) -> Exact<Decimal> {
    let (a, a_scale) = parts(numerator);
    let (b, b_scale) = parts(denominator);
    if b == BigInt::from(0) {
        return Err(crate::decimal::Refusal::DivisionByZero);
    }
    // a / b as integers over one scale: the ratio is unchanged.
    let common = a_scale.max(b_scale);
    let a = a * ten_to(common - a_scale);
    let b = b * ten_to(common - b_scale);
    if a == BigInt::from(0) {
        return Ok(Decimal::from_int(0));
    }
    let wanted = i64::try_from(crate::decimal::DIVISION_DIGITS).unwrap_or(100);
    // sqrt(a / b) = sqrt(a 10^(2k) / b) 10^-k, with k large enough that the
    // integer root has two digits more than wanted.
    let k = (wanted + 2 - (digits(&a) - digits(&b)) / 2 + 1).max(0);
    let scaled = a * ten_to(2 * k);
    let quotient = &scaled / &b;
    let mut exact = (&scaled % &b) == BigInt::from(0);
    let root = quotient.sqrt();
    exact = exact && &root * &root == quotient;
    let dropped = (digits(&root) - wanted).max(0);
    let divisor = ten_to(dropped);
    let mut kept = &root / &divisor;
    let tail = &root % &divisor;
    let half = &divisor / BigInt::from(2);
    // Past an inexact root the true value is above `root`, so a tail of
    // exactly half rounds up; an exact one rounds to even.
    let up = match tail.cmp(&half) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => !exact || (&kept % BigInt::from(2)) == BigInt::from(1),
    };
    if up && dropped > 0 {
        kept += 1;
    }
    let mut exponent = dropped - k;
    if exact && tail == BigInt::from(0) {
        while exponent < 0 && (&kept % BigInt::from(10)) == BigInt::from(0) {
            kept /= 10;
            exponent += 1;
        }
    }
    Decimal::parse(&format!("{kept}e{exponent}")).ok_or(crate::decimal::Refusal::OutOfRange)
}

// From states to statistics.

/// A statistic as a value: present, of its kind's type.
fn present(value: Value) -> Value {
    Value::Present(Arc::new(value))
}

/// A `Float` statistic, refused where it is not finite as Float arithmetic
/// is.
fn float(value: f64, span: &Range<usize>) -> Result<Value> {
    if value.is_finite() {
        Ok(present(Value::Float(value)))
    } else {
        Err(Diagnostic::overflow(span.clone()))
    }
}

/// An exact statistic, at its kind's type: the `Decimal`, or the nearest
/// `Float` to it.
fn exact(kind: Kind, value: Decimal, span: &Range<usize>) -> Result<Value> {
    match kind {
        Kind::Decimal => Ok(present(Value::Decimal(value))),
        Kind::Int | Kind::Float => match value.to_float() {
            Some(value) => float(value, span),
            None => Err(Diagnostic::overflow(span.clone())),
        },
    }
}

fn refused(span: &Range<usize>) -> impl Fn(crate::decimal::Refusal) -> Diagnostic + '_ {
    |refusal| refusal.diagnostic(span)
}

/// Which spread a statistic gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spread {
    Variance,
    Stdev,
    PVariance,
    PStdev,
}

impl Spread {
    fn name(self) -> &'static str {
        match self {
            Self::Variance => "variance",
            Self::Stdev => "stdev",
            Self::PVariance => "pvariance",
            Self::PStdev => "pstdev",
        }
    }

    fn sample(self) -> bool {
        matches!(self, Self::Variance | Self::Stdev)
    }

    fn root(self) -> bool {
        matches!(self, Self::Stdev | Self::PStdev)
    }

    /// The fewest values it is defined over.
    fn least(self) -> u64 {
        if self.sample() { 2 } else { 1 }
    }
}

fn spread(input: &Value, statistic: Spread, span: &Range<usize>) -> Result<Value> {
    let (kind, numbers) = numbers(input, statistic.name(), span)?;
    match numbers {
        Numbers::Float(values) => {
            let state = Moments::of(&values);
            if state.n < statistic.least() {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            #[allow(clippy::cast_precision_loss)]
            let divisor = if statistic.sample() {
                (state.n - 1) as f64
            } else {
                state.n as f64
            };
            let variance = state.m2 / divisor;
            float(
                if statistic.root() {
                    variance.sqrt()
                } else {
                    variance
                },
                span,
            )
        }
        Numbers::Int(values) => {
            let values: Vec<Decimal> = values.into_iter().map(Decimal::from_int).collect();
            exact_spread(kind, &values, statistic, span)
        }
        Numbers::Decimal(values) => exact_spread(kind, &values, statistic, span),
    }
}

fn exact_spread(
    kind: Kind,
    values: &[Decimal],
    statistic: Spread,
    span: &Range<usize>,
) -> Result<Value> {
    let state = Sums::of(values).map_err(refused(span))?;
    if u64::try_from(state.n).unwrap_or(0) < statistic.least() {
        return Ok(Value::Absent(kind.result()));
    }
    let numerator = state.spread_x().map_err(refused(span))?;
    let divisor = if statistic.sample() {
        state.sample_divisor()
    } else {
        state.population_divisor()
    };
    let value = if statistic.root() {
        sqrt_ratio(&numerator, &divisor)
    } else {
        numerator.div(&divisor)
    }
    .map_err(refused(span))?;
    exact(kind, value, span)
}

fn eval_mean(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (kind, numbers) = numbers(&input, "mean", &span)?;
    let values = match numbers {
        Numbers::Float(values) => {
            let state = Moments::of(&values);
            if state.n == 0 {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            return float(state.mean, &span);
        }
        Numbers::Int(values) => values.into_iter().map(Decimal::from_int).collect(),
        Numbers::Decimal(values) => values,
    };
    let state = Sums::of(&values).map_err(refused(&span))?;
    if state.n == 0 {
        return Ok(Value::Absent(kind.result()));
    }
    let mean = state.x.div(&state.count()).map_err(refused(&span))?;
    exact(kind, mean, &span)
}

fn eval_variance(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    spread(&input, Spread::Variance, &span)
}

fn eval_stdev(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    spread(&input, Spread::Stdev, &span)
}

fn eval_pvariance(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    spread(&input, Spread::PVariance, &span)
}

fn eval_pstdev(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    spread(&input, Spread::PStdev, &span)
}

fn eval_covariance(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (kind, pairs) = pairs(&input, "covariance", &span)?;
    match pairs {
        PairNumbers::Float(values) => {
            let state = CoMoments::of(&values);
            if state.x.n < 2 {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            #[allow(clippy::cast_precision_loss)]
            float(state.c / (state.x.n - 1) as f64, &span)
        }
        PairNumbers::Exact(values) => {
            let state = Sums::of_pairs(&values).map_err(refused(&span))?;
            if state.n < 2 {
                return Ok(Value::Absent(kind.result()));
            }
            let value = state
                .spread_xy()
                .and_then(|numerator| numerator.div(&state.sample_divisor()))
                .map_err(refused(&span))?;
            exact(kind, value, &span)
        }
    }
}

fn eval_correlation(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (_, pairs) = pairs(&input, "correlation", &span)?;
    match pairs {
        PairNumbers::Float(values) => {
            let state = CoMoments::of(&values);
            if state.x.n < 2 || state.x.m2 == 0.0 || state.y.m2 == 0.0 {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            float(state.c / (state.x.m2 * state.y.m2).sqrt(), &span)
        }
        PairNumbers::Exact(values) => {
            let state = Sums::of_pairs(&values).map_err(refused(&span))?;
            if state.n < 2 {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            let (sx, sy, sxy) =
                (|| Ok((state.spread_x()?, state.spread_y()?, state.spread_xy()?)))()
                    .map_err(refused(&span))?;
            if sx.is_zero() || sy.is_zero() {
                return Ok(Value::Absent(TypeRef::FLOAT));
            }
            // r = sxy / sqrt(sx sy), rounded once: the root of r² with the
            // sign of sxy.
            let magnitude = sxy
                .mul(&sxy)
                .and_then(|square| sqrt_ratio(&square, &sx.mul(&sy)?))
                .map_err(refused(&span))?;
            let r = if sxy.signum() < 0 {
                magnitude.negated()
            } else {
                magnitude
            };
            exact(Kind::Float, r, &span)
        }
    }
}

// Order statistics.

/// The values sorted ascending.
fn sorted(input: &Value, name: &str, span: &Range<usize>) -> Result<(Kind, Numbers)> {
    let (kind, numbers) = numbers(input, name, span)?;
    Ok((
        kind,
        match numbers {
            Numbers::Int(mut values) => {
                values.sort_unstable();
                Numbers::Int(values)
            }
            Numbers::Float(mut values) => {
                values.sort_by(f64::total_cmp);
                Numbers::Float(values)
            }
            Numbers::Decimal(mut values) => {
                values.sort();
                Numbers::Decimal(values)
            }
        },
    ))
}

/// The type-7 quantile of sorted `Float`s at `q`.
fn float_quantile(sorted: &[f64], q: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let h = (sorted.len() - 1) as f64 * q;
    let floor = h.floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = floor as usize;
    let low = sorted[at];
    match sorted.get(at + 1) {
        Some(high) if h > floor => low + (h - floor) * (high - low),
        _ => low,
    }
}

/// The type-7 quantile of sorted exact values at an exact `q`.
fn exact_quantile(sorted: &[Decimal], q: &Decimal) -> Exact<Decimal> {
    let last = i64::try_from(sorted.len() - 1).unwrap_or(i64::MAX);
    let h = Decimal::from_int(last).mul(q)?;
    let floor = h.to_int().unwrap_or(0);
    let fraction = h.sub(&Decimal::from_int(floor))?;
    let at = usize::try_from(floor).unwrap_or(0);
    let low = &sorted[at];
    match sorted.get(at + 1) {
        Some(high) if !fraction.is_zero() => low.add(&fraction.mul(&high.sub(low)?)?),
        _ => Ok(low.clone()),
    }
}

/// A quantile's `q`: a `Float`, or a `Decimal` read exactly.
#[derive(Debug, Clone)]
enum Q {
    Float(f64),
    Exact(Decimal),
}

impl Q {
    fn float(&self) -> f64 {
        match self {
            Self::Float(q) => *q,
            Self::Exact(q) => q.to_float().unwrap_or(f64::NAN),
        }
    }

    /// `q` exactly: a `Float` by its shortest decimal digits.
    fn exact(&self) -> Exact<Decimal> {
        match self {
            Self::Float(q) => Decimal::from_float(*q).ok_or(crate::decimal::Refusal::OutOfRange),
            Self::Exact(q) => Ok(q.clone()),
        }
    }
}

/// The quantile of a numeric collection at `q`, or `none` over nothing.
fn quantile_of(input: &Value, q: &Q, name: &str, span: &Range<usize>) -> Result<Value> {
    let (kind, numbers) = sorted(input, name, span)?;
    match numbers {
        Numbers::Float(values) if values.is_empty() => Ok(Value::Absent(TypeRef::FLOAT)),
        Numbers::Float(values) => float(float_quantile(&values, q.float()), span),
        Numbers::Int(values) => {
            let values: Vec<Decimal> = values.into_iter().map(Decimal::from_int).collect();
            exact_quantile_of(kind, &values, q, span)
        }
        Numbers::Decimal(values) => exact_quantile_of(kind, &values, q, span),
    }
}

fn exact_quantile_of(kind: Kind, values: &[Decimal], q: &Q, span: &Range<usize>) -> Result<Value> {
    if values.is_empty() {
        return Ok(Value::Absent(kind.result()));
    }
    let value = q
        .exact()
        .and_then(|q| exact_quantile(values, &q))
        .map_err(refused(span))?;
    exact(kind, value, span)
}

fn eval_median(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    quantile_of(&input, &Q::Float(0.5), "median", &span)
}

fn eval_quantile(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let argument = super::named_or_first(arguments, "q")
        .ok_or_else(|| fail(&span, "`quantile` needs a `q`: `quantile(0.95)`"))?;
    let q = match cx.evaluate(&argument.value)? {
        Value::Float(q) => Q::Float(q),
        Value::Decimal(q) => Q::Exact(q),
        _ => return Err(fail(&span, "`quantile` needs a Float or Decimal `q`")),
    };
    let inside = match &q {
        Q::Float(q) => (0.0..=1.0).contains(q),
        Q::Exact(q) => {
            q.signum() >= 0
                && q.sub(&Decimal::from_int(1))
                    .is_ok_and(|over| over.signum() <= 0)
        }
    };
    if !inside {
        let written = match &q {
            Q::Float(q) => format!("{q:?}"),
            Q::Exact(q) => q.to_string(),
        };
        return Err(fail(
            &argument.value.span,
            format!("`quantile` needs a `q` from 0 to 1, and {written} is outside it"),
        ));
    }
    quantile_of(&input, &q, "quantile", &span)
}

/// The `k - 1` cut points of sorted values, the `i`-th
/// `(s_j (k - d) + s_(j+1) d) / k` with `j = i (n - 1) div k` and
/// `d = i (n - 1) mod k`.
fn cut_points<T>(
    sorted: &[T],
    k: i64,
    mut point: impl FnMut(&T, Option<&T>, i64) -> Result<Value>,
) -> Result<Vec<Value>> {
    let last = i64::try_from(sorted.len().saturating_sub(1)).unwrap_or(i64::MAX);
    let mut cut = Vec::new();
    for i in 1..k {
        let position = i.checked_mul(last).unwrap_or(i64::MAX);
        let (j, d) = (position / k, position % k);
        let at = usize::try_from(j).unwrap_or(0);
        let next = if d == 0 { None } else { sorted.get(at + 1) };
        cut.push(point(&sorted[at], next, d)?);
    }
    Ok(cut)
}

fn eval_quantiles(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let argument = super::named_or_first(arguments, "n")
        .ok_or_else(|| fail(&span, "`quantiles` needs a count: `quantiles(4)`"))?;
    let Value::Int(k) = cx.evaluate(&argument.value)? else {
        return Err(fail(&span, "`quantiles` needs an Int count"));
    };
    if k < 2 {
        return Err(fail(
            &argument.value.span,
            format!("`quantiles` needs a count of 2 or more, and {k} is below 2"),
        ));
    }
    let (kind, numbers) = sorted(&input, "quantiles", &span)?;
    let exact_points = |values: &[Decimal]| -> Result<Vec<Value>> {
        let whole = Decimal::from_int(k);
        cut_points(values, k, |low, high, d| {
            let value = match high {
                None => Ok(low.clone()),
                Some(high) => low
                    .mul(&Decimal::from_int(k - d))
                    .and_then(|left| left.add(&high.mul(&Decimal::from_int(d))?))
                    .and_then(|sum| sum.div(&whole)),
            }
            .map_err(refused(&span))?;
            match kind {
                Kind::Decimal => Ok(Value::Decimal(value)),
                Kind::Int | Kind::Float => value
                    .to_float()
                    .map(Value::Float)
                    .ok_or_else(|| Diagnostic::overflow(span.clone())),
            }
        })
    };
    let cut = match numbers {
        Numbers::Float(values) if values.is_empty() => Vec::new(),
        Numbers::Int(values) if values.is_empty() => Vec::new(),
        Numbers::Decimal(values) if values.is_empty() => Vec::new(),
        Numbers::Float(values) => {
            #[allow(clippy::cast_precision_loss)]
            let whole = k as f64;
            cut_points(&values, k, |low, high, d| {
                #[allow(clippy::cast_precision_loss)]
                let value = match high {
                    None => *low,
                    Some(high) => (low * (k - d) as f64 + high * d as f64) / whole,
                };
                if value.is_finite() {
                    Ok(Value::Float(value))
                } else {
                    Err(Diagnostic::overflow(span.clone()))
                }
            })?
        }
        Numbers::Int(values) => {
            let values: Vec<Decimal> = values.into_iter().map(Decimal::from_int).collect();
            exact_points(&values)?
        }
        Numbers::Decimal(values) => exact_points(&values)?,
    };
    Ok(Value::list(cut, kind.result()))
}

fn eval_mode(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let element = element_type(&input);
    let mut held = elements(&input, "mode", &span)?;
    if held.is_empty() {
        return Ok(Value::Absent(element));
    }
    // Sorted ascending, equal values are neighbours, and the first longest
    // run is the least of the most frequent.
    in_element_order(cx, &mut held, &span)?;
    let mut best = (0, 0);
    let mut start = 0;
    while start < held.len() {
        let mut end = start + 1;
        while end < held.len()
            && cx.compare_values(&held[start], &held[end], &span)? == Ordering::Equal
        {
            end += 1;
        }
        if end - start > best.1 {
            best = (start, end - start);
        }
        start = end;
    }
    Ok(present(held.swap_remove(best.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    /// The fold of `values` split into `parts` contiguous partitions, each
    /// folded alone and the states combined in order.
    fn split<T: Clone, S>(
        values: &[T],
        parts: usize,
        fold: impl Fn(&[T]) -> S,
        combine: impl Fn(S, S) -> S,
    ) -> S {
        let size = values.len().div_ceil(parts).max(1);
        values
            .chunks(size)
            .map(&fold)
            .reduce(&combine)
            .unwrap_or_else(|| fold(&[]))
    }

    fn ulps(a: f64, b: f64) -> u64 {
        a.to_bits().abs_diff(b.to_bits())
    }

    const FLOATS: [f64; 11] = [
        2.75, 1.75, 1.25, 0.25, 0.5, 1.25, 3.5, 10.125, -4.5, 7.0, 0.1,
    ];

    #[test]
    fn a_float_reduction_split_is_within_four_ulps_of_the_whole() {
        let whole = Moments::of(&FLOATS);
        for parts in [1, 2, 7] {
            let state = split(&FLOATS, parts, Moments::of, Moments::combine);
            assert_eq!(state.n, whole.n);
            assert!(ulps(state.mean, whole.mean) <= 4, "{parts}: {state:?}");
            assert!(ulps(state.m2, whole.m2) <= 4, "{parts}: {state:?}");
        }
        let pairs: Vec<(f64, f64)> = FLOATS.iter().map(|x| (*x, x * x - 1.0)).collect();
        let whole = CoMoments::of(&pairs);
        for parts in [1, 2, 7] {
            let state = split(&pairs, parts, CoMoments::of, CoMoments::combine);
            assert!(ulps(state.c, whole.c) <= 4, "{parts}: {state:?}");
            assert!(ulps(state.y.m2, whole.y.m2) <= 4, "{parts}: {state:?}");
        }
    }

    #[test]
    fn an_exact_reduction_split_is_the_whole() {
        let values: Vec<Decimal> = ["2.75", "1.75", "1.25", "0.25", "0.5", "1.25", "3.5", "0.1"]
            .iter()
            .map(|text| d(text))
            .collect();
        let whole = Sums::of(&values).unwrap();
        for parts in [1, 2, 7] {
            let state = split(
                &values,
                parts,
                |chunk| Sums::of(chunk).unwrap(),
                |a, b| a.combine(&b).unwrap(),
            );
            assert_eq!(state, whole, "{parts}");
        }
        let pairs: Vec<(Decimal, Decimal)> = values
            .iter()
            .map(|x| (x.clone(), x.mul(x).unwrap()))
            .collect();
        let whole = Sums::of_pairs(&pairs).unwrap();
        for parts in [1, 2, 7] {
            let state = split(
                &pairs,
                parts,
                |chunk| Sums::of_pairs(chunk).unwrap(),
                |a, b| a.combine(&b).unwrap(),
            );
            assert_eq!(state, whole, "{parts}");
        }
    }

    #[test]
    fn nan_propagates_through_the_float_state() {
        let state = Moments::of(&[1.0, f64::NAN, 3.0]);
        assert!(state.mean.is_nan() && state.m2.is_nan());
        assert!(CoMoments::of(&[(1.0, 2.0), (f64::NAN, 1.0)]).c.is_nan());
        assert!(float_quantile(&[1.0, f64::NAN], 0.5).is_nan());
    }

    #[test]
    fn a_square_root_is_rounded_once_and_exact_where_it_terminates() {
        assert_eq!(sqrt_ratio(&d("2.25"), &d("1")).unwrap().to_string(), "1.5");
        assert_eq!(sqrt_ratio(&d("9"), &d("4")).unwrap().to_string(), "1.5");
        assert_eq!(sqrt_ratio(&d("0"), &d("4")).unwrap().to_string(), "0");
        assert_eq!(
            sqrt_ratio(&d("1e40"), &d("1")).unwrap().to_string(),
            format!("1{}", "0".repeat(20))
        );
        // Python: Decimal(2).sqrt() under a context of 100 digits.
        assert_eq!(
            sqrt_ratio(&d("2"), &d("1")).unwrap().to_string(),
            "1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573"
        );
        // Python: (Decimal(1) / 3).sqrt() of the exact third, 100 digits.
        assert_eq!(
            sqrt_ratio(&d("1"), &d("3")).unwrap().to_string(),
            "0.5773502691896257645091487805019574556476017512701268760186023264839776723029333456937153955857495252"
        );
    }
}
