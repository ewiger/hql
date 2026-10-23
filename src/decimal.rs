//! `Decimal`: an exact decimal number, an integer significand of any size and
//! a scale (HQL-0031).
//!
//! The scale is the number of digits after the point. It is kept for
//! display, so `1.50` prints as `1.50`, and never below 0. Equality and order
//! are by value, so `1.0 == 1.00`.

use bigdecimal::num_bigint::{BigInt, Sign};
use bigdecimal::{BigDecimal, RoundingMode, Signed, ToPrimitive, Zero};
use std::fmt;
use std::str::FromStr;

/// How many significant digits a quotient that does not terminate keeps.
pub const DIVISION_DIGITS: u64 = 100;

/// An exact decimal number with the scale it was written or computed with.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decimal(BigDecimal);

impl Decimal {
    /// A value whose scale is raised to 0 when it is negative, so `1.5e3`
    /// holds `1500`.
    fn scaled(value: BigDecimal) -> Self {
        let (_, scale) = value.as_bigint_and_exponent();
        if scale < 0 {
            Self(value.with_scale(0))
        } else {
            Self(value)
        }
    }

    /// Read decimal text: an optional sign, digits with an optional point,
    /// and an optional exponent. Answers `None` for anything else.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let body = text.strip_prefix(['-', '+']).unwrap_or(text);
        let (mantissa, exponent) = match body.find(['e', 'E']) {
            Some(at) => (&body[..at], Some(&body[at + 1..])),
            None => (body, None),
        };
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if whole.is_empty() && fraction.is_empty()
            || !digits(whole)
            || !digits(fraction)
            || exponent.is_some_and(|exponent| {
                let exponent = exponent.strip_prefix(['-', '+']).unwrap_or(exponent);
                exponent.is_empty() || !digits(exponent)
            })
        {
            return None;
        }
        BigDecimal::from_str(text).ok().map(Self::scaled)
    }

    /// The scale: how many digits follow the point.
    #[must_use]
    pub fn scale(&self) -> i64 {
        self.0.as_bigint_and_exponent().1
    }

    /// The value of an `Int`, scale 0.
    #[must_use]
    pub fn from_int(value: i64) -> Self {
        Self(BigDecimal::from(value))
    }

    /// The value of a finite `Float`, by the shortest digits that read back
    /// as it. `None` for a value that is not finite.
    #[must_use]
    pub fn from_float(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        // `{:?}` writes the shortest digits that round-trip, with a point.
        Self::parse(&format!("{value:?}"))
    }

    /// The nearest `Float`, or `None` past the finite range.
    #[must_use]
    pub fn to_float(&self) -> Option<f64> {
        self.0.to_f64().filter(|value| value.is_finite())
    }

    /// The integer part, the fraction dropped toward zero, or `None` outside
    /// the `Int` range.
    #[must_use]
    pub fn to_int(&self) -> Option<i64> {
        self.0
            .with_scale_round(0, RoundingMode::Down)
            .as_bigint_and_exponent()
            .0
            .to_i64()
    }

    /// Whether the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// The value without its sign, at the same scale.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// The value with its sign reversed, at the same scale.
    #[must_use]
    pub fn negated(&self) -> Self {
        Self(-self.0.clone())
    }

    /// -1, 0 or 1 by the sign of the value.
    #[must_use]
    pub fn signum(&self) -> i64 {
        match self.0.sign() {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        }
    }

    /// The sum, at the larger of the two scales.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        Self::scaled(&self.0 + &other.0).at_least(self.scale().max(other.scale()))
    }

    /// The difference, at the larger of the two scales.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        Self::scaled(&self.0 - &other.0).at_least(self.scale().max(other.scale()))
    }

    /// The product, at the sum of the two scales.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        Self::scaled(&self.0 * &other.0).at_least(self.scale() + other.scale())
    }

    /// The remainder of a division truncated toward zero, with the sign of
    /// the dividend, at the larger of the two scales. `None` when `other`
    /// is zero.
    #[must_use]
    pub fn rem(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        let scale = self.scale().max(other.scale());
        let (a, _) = self.0.with_scale(scale).into_bigint_and_exponent();
        let (b, _) = other.0.with_scale(scale).into_bigint_and_exponent();
        Some(Self(BigDecimal::new(a % b, scale)))
    }

    /// The quotient: exact where it terminates, and otherwise rounded half
    /// to even to [`DIVISION_DIGITS`] significant digits. `None` when
    /// `other` is zero.
    #[must_use]
    pub fn div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        let (a, a_scale) = self.0.as_bigint_and_exponent();
        let (b, b_scale) = other.0.as_bigint_and_exponent();
        let negative = (a.sign() == Sign::Minus) != (b.sign() == Sign::Minus);
        let (a, b) = (a.abs(), b.abs());
        // The quotient is a / b * 10^(b_scale - a_scale).
        let shift = a_scale - b_scale;
        let common = num_gcd(&a, &b);
        let (numerator, denominator) = (&a / &common, &b / &common);
        let (significand, scale) = match terminating(&numerator, &denominator) {
            Some((significand, digits)) => (significand, digits + shift),
            None => rounded_quotient(&numerator, &denominator, shift),
        };
        let significand = if negative { -significand } else { significand };
        Some(Self::scaled(BigDecimal::new(significand, scale)))
    }

    /// The value rounded half to even to `places` digits after the point,
    /// at scale `places`, or at scale 0 when `places` is negative.
    #[must_use]
    pub fn round(&self, places: i64) -> Self {
        Self::scaled(self.0.with_scale_round(places, RoundingMode::HalfEven))
    }

    /// The same value at `scale` or more digits after the point.
    fn at_least(self, scale: i64) -> Self {
        if self.scale() < scale {
            Self(self.0.with_scale(scale))
        } else {
            self
        }
    }

    /// The value with its trailing zeros removed, which every equal value
    /// shares: what a digest of it is taken over.
    #[must_use]
    pub fn canonical(&self) -> String {
        if self.0.is_zero() {
            return "0".to_owned();
        }
        Self::scaled(self.0.normalized()).to_string()
    }
}

/// The greatest common divisor of two non-negative integers, 1 for two zeros.
fn num_gcd(a: &BigInt, b: &BigInt) -> BigInt {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() {
        let next = &a % &b;
        a = b;
        b = next;
    }
    if a.is_zero() { BigInt::from(1) } else { a }
}

/// `numerator / denominator` as a significand and its count of digits
/// after the point, when the denominator has no prime factor but 2 and 5.
fn terminating(numerator: &BigInt, denominator: &BigInt) -> Option<(BigInt, i64)> {
    let (two, five) = (BigInt::from(2), BigInt::from(5));
    let mut rest = denominator.clone();
    let (mut twos, mut fives) = (0_u32, 0_u32);
    while (&rest % &two).is_zero() {
        rest /= &two;
        twos += 1;
    }
    while (&rest % &five).is_zero() {
        rest /= &five;
        fives += 1;
    }
    if rest != BigInt::from(1) {
        return None;
    }
    let digits = twos.max(fives);
    let scaled = numerator * BigInt::from(10).pow(digits) / denominator;
    Some((scaled, i64::from(digits)))
}

/// `numerator / denominator * 10^-shift` rounded half to even to
/// [`DIVISION_DIGITS`] significant digits, as a significand and a scale.
/// The quotient does not terminate, so its remainder is never zero.
fn rounded_quotient(numerator: &BigInt, denominator: &BigInt, shift: i64) -> (BigInt, i64) {
    let length = |value: &BigInt| i64::try_from(value.to_string().len()).unwrap_or(i64::MAX);
    let wanted = i64::try_from(DIVISION_DIGITS).unwrap_or(i64::MAX);
    // Enough places that the integer quotient has more digits than wanted.
    let places = (wanted + 2 - (length(numerator) - length(denominator))).max(0);
    let power = BigInt::from(10).pow(u32::try_from(places).unwrap_or(u32::MAX));
    let scaled = numerator * power;
    let quotient = &scaled / denominator;
    let sticky = !(&scaled % denominator).is_zero();
    let dropped = (length(&quotient) - wanted).max(0);
    let divisor = BigInt::from(10).pow(u32::try_from(dropped).unwrap_or(u32::MAX));
    let mut kept = &quotient / &divisor;
    let tail = &quotient % &divisor;
    let half = &divisor / BigInt::from(2);
    let up = match tail.cmp(&half) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        // The digits past the tail are not all zero, so it is above half.
        std::cmp::Ordering::Equal if sticky => true,
        std::cmp::Ordering::Equal => (&kept % BigInt::from(2)) == BigInt::from(1),
    };
    if up {
        kept += 1;
    }
    (kept, places - dropped + shift)
}

impl FromStr for Decimal {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text).ok_or_else(|| format!("{text:?} is not a Decimal"))
    }
}

impl fmt::Display for Decimal {
    /// The digits, with as many after the point as the scale says.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_plain_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    #[test]
    fn a_literal_keeps_its_scale_and_every_digit() {
        assert_eq!(d("12.50").to_string(), "12.50");
        assert_eq!(d("1.5e-3").to_string(), "0.0015");
        assert_eq!(d("1.5e3").to_string(), "1500");
        assert_eq!(
            d("123456789012345678901234567890.123456789").to_string(),
            "123456789012345678901234567890.123456789"
        );
        assert_eq!(d("1.0"), d("1.00"));
        assert!(Decimal::parse("1.2.3").is_none());
        assert!(Decimal::parse("e5").is_none());
    }

    #[test]
    fn arithmetic_states_its_scale() {
        assert_eq!(d("0.1").add(&d("0.2")).to_string(), "0.3");
        assert_eq!(d("1.50").add(&d("1")).to_string(), "2.50");
        assert_eq!(d("1.50").sub(&d("1.50")).to_string(), "0.00");
        assert_eq!(d("1.5").mul(&d("2.25")).to_string(), "3.375");
        assert_eq!(d("2.0").mul(&d("3.00")).to_string(), "6.000");
        assert_eq!(d("7.5").rem(&d("2")).unwrap().to_string(), "1.5");
    }

    #[test]
    fn division_is_exact_where_it_terminates() {
        assert_eq!(d("1").div(&d("4")).unwrap().to_string(), "0.25");
        assert_eq!(d("1.00").div(&d("4")).unwrap().to_string(), "0.25");
        assert_eq!(d("10").div(&d("4")).unwrap().to_string(), "2.5");
        assert_eq!(d("6").div(&d("3")).unwrap().to_string(), "2");
        assert_eq!(d("-1").div(&d("8")).unwrap().to_string(), "-0.125");
        assert!(d("1").div(&d("0")).is_none());
    }

    #[test]
    fn division_that_does_not_terminate_keeps_a_hundred_digits() {
        let third = d("1").div(&d("3")).unwrap().to_string();
        assert_eq!(third, format!("0.{}", "3".repeat(100)));
        let two_thirds = d("2").div(&d("3")).unwrap().to_string();
        assert_eq!(two_thirds, format!("0.{}7", "6".repeat(99)));
        let big = d("100").div(&d("3")).unwrap().to_string();
        assert_eq!(big, format!("33.{}", "3".repeat(98)));
    }

    #[test]
    fn round_is_half_to_even() {
        assert_eq!(d("2.345").round(2).to_string(), "2.34");
        assert_eq!(d("2.355").round(2).to_string(), "2.36");
        assert_eq!(d("2.5").round(0).to_string(), "2");
        assert_eq!(d("3.5").round(0).to_string(), "4");
        assert_eq!(d("1.2").round(3).to_string(), "1.200");
    }

    #[test]
    fn conversions_to_and_from_the_other_numbers() {
        assert_eq!(Decimal::from_float(0.1).unwrap().to_string(), "0.1");
        assert_eq!(Decimal::from_int(-7).to_string(), "-7");
        assert_eq!(d("-2.75").to_int(), Some(-2));
        assert_eq!(d("2.5").to_float(), Some(2.5));
        assert_eq!(d("1.10").canonical(), d("1.1000").canonical());
    }
}
