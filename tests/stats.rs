//! The `stats` module (HQL-0041 part 2): each statistic against Python's
//! `statistics` over a fixed list of `Int`, `Float` and `Decimal` values,
//! `quantile` and `quantiles` against `quantiles(method="inclusive")`, the
//! empty and short inputs, the mode's ties, and the refusals.
//!
//! Every expected value was computed with Python 3.14's `statistics`, a
//! `Decimal` one under a context of 100 digits. Where the two differ in the
//! last bits of a `Float`, the test allows 4 ulps and says so.

fn shown(source: &str) -> String {
    hql::eval(&format!("import stats\n{source}"))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refused(source: &str) -> String {
    hql::eval(&format!("import stats\n{source}"))
        .map(|value| panic!("{source} gave {value}"))
        .unwrap_err()
        .message
}

const INTS: &str = "[2, 4, 4, 4, 5, 5, 7, 9]";
const FLOATS: &str = "[0.5, 1.25, 1.25, 1.75, 2.75, 3.5, 10.125, -4.5] | map(x => float(x))";
const DECIMALS: &str = "[1.10, 2.20, 3.30, 4.45, 5.05]";

/// Each statistic of `values`, in the order the list below names them.
fn statistics(values: &str) -> String {
    shown(&format!(
        "xs = {values}\n\
         [xs | mean, xs | stats.variance, xs | stdev, xs | pvariance, xs | pstdev, xs | median]"
    ))
}

#[test]
fn int_statistics_are_pythons() {
    // mean 5, variance 4.571428571428571, stdev 2.138089935299395,
    // pvariance 4, pstdev 2.0, median 4.5: each the exact value rounded once.
    assert_eq!(
        statistics(INTS),
        "[5.0, 4.571428571428571, 2.138089935299395, 4.0, 2.0, 4.5]"
    );
}

#[test]
fn float_statistics_are_pythons() {
    assert_eq!(
        statistics(FLOATS),
        "[2.078125, 16.352399553571427, 4.043810029362337, 14.308349609375, \
         3.7826379167685347, 1.5]"
    );
}

#[test]
fn decimal_statistics_are_pythons_to_100_digits() {
    assert_eq!(
        statistics(DECIMALS),
        "[3.22, 2.60075, \
         1.612684098017959622740596234226343645204362903449989187063601000900849448193938189822661400192148978, \
         2.0806, \
         1.442428507760436631616618531408285319387137306209957654198367521396137223519270574802423317112105951, \
         3.30]"
    );
    // A Decimal statistic stays a Decimal, an Int or Float one is a Float.
    assert_eq!(
        hql::check(&format!("import stats\n{DECIMALS} | stdev"))
            .unwrap()
            .to_string(),
        "Option<Decimal>"
    );
    assert_eq!(
        hql::check(&format!("import stats\n{INTS} | mean"))
            .unwrap()
            .to_string(),
        "Option<Float>"
    );
}

#[test]
fn quantiles_are_pythons_inclusive_method() {
    for (values, expected) in [
        (
            INTS,
            "[[4.0, 4.5, 5.5], [3.4, 4.0, 4.0, 4.0, 4.5, 5.0, 5.0, 6.2, 7.6]]",
        ),
        (
            FLOATS,
            "[[1.0625, 1.5, 2.9375], [-1.0, 0.8, 1.25, 1.25, 1.5, 1.95, 2.65, 3.2, 5.4875]]",
        ),
        (
            DECIMALS,
            "[[2.20, 3.30, 4.45], [1.54, 1.98, 2.42, 2.86, 3.30, 3.76, 4.22, 4.57, 4.81]]",
        ),
    ] {
        assert_eq!(
            shown(&format!(
                "xs = {values}\n[xs | quantiles(4), xs | quantiles(10)]"
            )),
            expected,
            "{values}"
        );
    }
}

#[test]
fn quantile_interpolates_type_7() {
    // statistics.quantiles([1, 2, 3, 4], n=20, method="inclusive")[18] is
    // 3.85: exact over Int and Decimal, a literal `q` read exactly.
    assert_eq!(shown("[1, 2, 3, 4] | quantile(0.95)"), "3.85");
    assert_eq!(shown("[4, 1, 3, 2] | quantile(float(0.95))"), "3.85");
    assert_eq!(shown("[1.0, 2.0, 3.0, 4.0] | quantile(0.95)"), "3.850");
    // Over Floats, h = (n - 1) q in Float arithmetic: within 4 ulps of 3.85.
    let float: f64 = shown("[1, 2, 3, 4] | map(x => float(x)) | quantile(0.95)")
        .parse()
        .unwrap();
    assert!(float.to_bits().abs_diff(3.85_f64.to_bits()) <= 4, "{float}");
    assert_eq!(shown("[3, 1, 4, 2] | median"), "2.5");
    assert_eq!(shown("[3, 1, 2] | quantile(0.0)"), "1.0");
    assert_eq!(shown("[3, 1, 2] | quantile(1.0)"), "3.0");
    assert_eq!(shown("[7] | quantiles(4)"), "[7.0, 7.0, 7.0]");
}

#[test]
fn covariance_and_correlation_are_pythons() {
    // covariance 3.2 and correlation 0.7917946548886297.
    assert_eq!(
        shown("p = zip([1, 2, 3, 4, 5, 6], [2, 1, 4, 3, 7, 5])\n[p | covariance, p | correlation]"),
        "[3.2, 0.7917946548886297]"
    );
    // Decimal: the covariance exact, the correlation 2.5 / sqrt(7) rounded once.
    assert_eq!(
        shown("p = zip([1.5, 2.5, 3.5], [2.0, 2.5, 4.5])\n[p | covariance, p | correlation]"),
        "[1.25, 0.944911182523068]"
    );
    // Float: covariance 1.4322916666666667 and correlation
    // 0.8909760045573636, which the running co-moments give within 4 ulps.
    let float = "p = zip([0.5, 1.5, 2.25, 3.0] | map(x => float(x)), \
                 [1.0, 0.75, 2.5, 4.0] | map(x => float(x)))\n";
    assert_eq!(
        shown(&format!("{float}p | covariance")),
        "1.4322916666666667"
    );
    let r: f64 = shown(&format!("{float}p | correlation")).parse().unwrap();
    assert!(
        r.to_bits().abs_diff(0.890_976_004_557_363_6_f64.to_bits()) <= 4,
        "{r}"
    );
    // Pairs written with the constructor.
    assert_eq!(
        shown("[Pair(first = 1, second = 2), Pair(first = 2, second = 4)] | covariance"),
        "1.0"
    );
}

#[test]
fn undefined_statistics_are_none() {
    for empty in [
        "mean",
        "pvariance",
        "pstdev",
        "median",
        "quantile(0.5)",
        "mode",
    ] {
        assert_eq!(
            shown(&format!("[1] | filter(x => x > 1) | {empty}")),
            "none",
            "{empty}"
        );
    }
    assert_eq!(shown("[1] | filter(x => x > 1) | quantiles(4)"), "[]");
    for short in ["stats.variance", "stdev"] {
        assert_eq!(shown(&format!("[4] | {short}")), "none", "{short}");
        assert_eq!(shown(&format!("[4.5] | {short}")), "none", "{short}");
    }
    assert_eq!(shown("[4] | pvariance"), "0.0");
    assert_eq!(shown("zip([1], [2]) | covariance"), "none");
    assert_eq!(shown("zip([1], [2]) | correlation"), "none");
    // Zero variance on either side leaves the correlation undefined.
    assert_eq!(shown("zip([1, 2, 3], [5, 5, 5]) | correlation"), "none");
    assert_eq!(shown("zip([2.5, 2.5], [1.0, 3.0]) | correlation"), "none");
    assert_eq!(
        shown("zip([1, 1] | map(x => float(x)), [1, 2] | map(x => float(x))) | correlation"),
        "none"
    );
}

#[test]
fn mode_ties_resolve_to_the_least_value() {
    // Python's mode gives 3, the first met; the least of the tied is 1.
    assert_eq!(shown("[3, 1, 3, 1, 2] | mode"), "1");
    assert_eq!(shown("[3, 1, 3, 2] | mode"), "3");
    assert_eq!(shown("[\"b\", \"a\", \"b\", \"a\"] | mode"), "a");
    assert_eq!(shown("[2.50, 2.5, 1.0] | mode"), "2.50");
    assert_eq!(shown("Set([3, 1, 2]) | mode"), "1");
}

#[test]
fn a_set_and_a_stream_are_collections() {
    assert_eq!(shown("Set([1, 2, 2, 3]) | mean"), "2.0");
    assert_eq!(shown("range(1, 5) | mean"), "2.5");
    assert_eq!(shown("[1, 2, 3, 4].mean()"), "2.5");
}

#[test]
fn a_quantile_outside_zero_to_one_and_fewer_than_two_groups_are_refused() {
    assert!(refused("[1, 2, 3] | quantile(1.5)").contains("1.5 is outside it"));
    assert!(refused("[1, 2, 3] | quantile(float(-0.5))").contains("-0.5 is outside it"));
    assert!(refused("[1, 2, 3] | quantiles(1)").contains("1 is below 2"));
    assert!(refused("[1, 2, 3] | quantiles(0)").contains("0 is below 2"));
}

#[test]
fn a_float_statistic_past_the_float_range_is_an_overflow() {
    let message = refused(
        "[\"1e300\", \"-1e300\", \"1e300\"] | map(s => s.to_float() | unwrap_or(float(0))) \
         | stats.variance",
    );
    assert!(message.contains("overflow"), "{message}");
}

#[test]
fn the_preludes_variance_and_stats_variance_are_told_apart_by_qualifying() {
    // The prelude's `variance` is the population variance of Ints and
    // Floats; importing `stats` makes a bare call over them ambiguous.
    let message = refused("[1, 2, 3, 4] | variance");
    assert!(message.contains("ambiguous"), "{message}");
    assert_eq!(shown("[1, 2, 3, 4] | stats.variance"), "1.6666666666666667");
    assert_eq!(shown("[1, 2, 3, 4] | core.variance"), "1.25");
    // Over Decimals only `stats` declares it.
    assert_eq!(shown("[1.5, 2.5] | variance"), "0.50");
}

#[test]
fn stats_is_imported_not_in_the_prelude() {
    let refused = hql::eval("[1, 2] | stdev").unwrap_err();
    assert!(
        refused.message.contains("import stats"),
        "{}",
        refused.message
    );
}

#[test]
fn the_reductions_are_reductions_and_the_order_statistics_barriers() {
    let directory = tempfile::tempdir().unwrap();
    for (step, shape) in [
        ("mean", "reduction"),
        ("stdev", "reduction"),
        ("pvariance", "reduction"),
        ("correlation", "reduction"),
        ("median", "barrier"),
        ("quantiles(4)", "barrier"),
        ("mode", "barrier"),
    ] {
        let path = directory.path().join("p.hql");
        let input = if step == "correlation" {
            "zip([1, 2, 3], [3, 1, 2]) | map(p => p)"
        } else {
            "[1, 2, 3] | map(x => x + 1)"
        };
        std::fs::write(&path, format!("import stats\n{input} | {step}\n")).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_hql"))
            .args(["plan", path.to_str().unwrap()])
            .output()
            .unwrap();
        let plan = String::from_utf8(output.stdout).unwrap();
        assert!(output.status.success(), "{plan}");
        let name = step.split('(').next().unwrap_or(step);
        let line = plan
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("| {name} ")))
            .unwrap_or_else(|| panic!("no {name} in {plan}"));
        assert!(line.contains(&format!("{shape}, pure")), "{step}: {line}");
    }
}
