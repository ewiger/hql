//! The `random` module (HQL-0041 part 4): seeded draws pinned to their
//! output, unseeded draws under `--seed`, the evaluation seed reported, the
//! edge cases of each operation, UUIDs, sampling a stream, and the purity of
//! each form.

use std::io::Write;
use std::process::{Command, Stdio};

fn shown(source: &str) -> String {
    hql::eval(&format!("import random\n{source}"))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .to_string()
}

fn refused(source: &str) -> String {
    hql::eval(&format!("import random\n{source}"))
        .map(|value| panic!("{source} gave {value}"))
        .unwrap_err()
        .message
}

/// What `hql` writes for `arguments` with `input` on standard input: its
/// standard output and standard error.
fn hql_with(arguments: &[&str], input: &str) -> (String, String) {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{error}");
    (String::from_utf8(output.stdout).unwrap(), error)
}

fn hql(arguments: &[&str]) -> String {
    hql_with(arguments, "").0
}

const TEN: &str = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]";

#[test]
fn each_seeded_form_gives_its_pinned_output() {
    // The generator is pinned (ChaCha12 keyed through SplitMix64), and so
    // is every draw from it, so these hold on every host and release.
    for (source, expected) in [
        (format!("{TEN} | sample(3, seed = 7)"), "[2, 4, 8]"),
        (format!("{TEN} | sample(3, seed = -5)"), "[5, 6, 10]"),
        (
            "[1, 2, 3, 4, 5] | shuffle(seed = 42)".to_owned(),
            "[1, 2, 3, 5, 4]",
        ),
        (format!("{TEN} | choice(seed = 7)"), "8"),
        (
            "Set([5, 3, 9, 1, 7]) | sample(2, seed = 11)".to_owned(),
            "[1, 3]",
        ),
        (format!("{TEN}.sample(3, seed = 7)"), "[1, 6, 7]"),
    ] {
        assert_eq!(shown(&source), expected, "{source}");
        assert_eq!(shown(&source), expected, "{source}, again");
    }
}

#[test]
fn a_seed_and_a_call_site_key_a_draw() {
    // Another seed draws another permutation; so does another call site.
    let first = shown("range(0, 20) | shuffle(seed = 1)");
    assert_ne!(first, shown("range(0, 20) | shuffle(seed = 2)"));
    assert_ne!(first, shown(" range(0, 20) | shuffle(seed = 1)"));
    // One call site applied twice draws the same, since the draw is pure.
    assert_eq!(
        shown("[range(0, 20), range(0, 20)] | map(xs => xs | shuffle(seed = 1)) | distinct | size"),
        "1"
    );
}

#[test]
fn a_set_built_in_two_orders_gives_one_sample() {
    assert_eq!(
        shown("[Set([5, 3, 9, 1, 7]), Set([7, 1, 9, 3, 5])] | map(s => s | sample(2, seed = 11))"),
        "[[1, 3], [1, 3]]"
    );
}

#[test]
fn a_sample_keeps_input_order_and_draws_without_replacement() {
    assert_eq!(
        shown("s = range(0, 100) | sample(10, seed = 3)\n[s == sort(s), size(Set(s))]"),
        "[true, 10]"
    );
    assert_eq!(
        shown("[1, 2, 3, 4, 5] | shuffle(seed = 42) | sort"),
        "[1, 2, 3, 4, 5]"
    );
}

#[test]
fn edge_cases_give_the_whole_input_nothing_or_an_error() {
    assert_eq!(shown("[3, 1, 2] | sample(5, seed = 1)"), "[3, 1, 2]");
    assert_eq!(shown("[3, 1, 2] | sample(3)"), "[3, 1, 2]");
    assert_eq!(shown("[3, 1, 2] | sample(0, seed = 1)"), "[]");
    assert_eq!(shown("[3, 1, 2] | sample(0)"), "[]");
    assert!(refused("[3, 1, 2] | sample(-1, seed = 1)").contains("below zero"));
    assert!(refused("[3, 1, 2] | sample(-1)").contains("below zero"));
    assert_eq!(
        shown("[1, 2] | filter(n => n > 5) | choice(seed = 1)"),
        "none"
    );
    assert_eq!(shown("[1, 2] | filter(n => n > 5) | choice()"), "none");
    assert_eq!(shown("[] | shuffle(seed = 1)"), "[]");
    assert!(refused("random_int(6, 1)").contains("1 is below 6"));
    assert_eq!(shown("random_int(4, 4)"), "4");
}

#[test]
fn unseeded_draws_stay_in_their_bounds() {
    assert_eq!(
        shown(
            "xs = range(0, 200) | map(_ => random_int(-2, 2))\n\
             [xs | all(n => n >= -2 and n <= 2), size(Set(xs))]"
        ),
        "[true, 5]"
    );
    assert_eq!(
        shown(
            "xs = range(0, 200) | map(_ => random_float())\nxs | all(x => x >= float(0.0) and x < float(1.0))"
        ),
        "true"
    );
    assert_eq!(shown("size([1, 2, 3, 4] | sample(2))"), "2");
    assert_eq!(shown("[1, 2, 3] | shuffle() | sort"), "[1, 2, 3]");
}

#[test]
fn an_unseeded_draw_under_seed_is_the_seeded_draw() {
    let unseeded = format!("import random\n{TEN} | sample(3)");
    let seeded = format!("import random\n{TEN} | sample(3, seed = 7)");
    assert_eq!(hql(&["--seed", "7", "eval", &unseeded]), "[2, 4, 8]\n");
    assert_eq!(hql(&["eval", &seeded]), "[2, 4, 8]\n");
    assert_eq!(hql(&["--seed", "-5", "eval", &unseeded]), "[5, 6, 10]\n");
    // Each call site carries on drawing, so repeated calls differ, and the
    // whole evaluation repeats under one seed.
    let repeated = "import random\nrange(0, 6) | map(_ => random_int(1, 1000))";
    let first = hql(&["--seed", "3", "eval", repeated]);
    assert_eq!(first, hql(&["--seed", "3", "eval", repeated]));
    assert_ne!(first, hql(&["--seed", "4", "eval", repeated]));
    let drawn: std::collections::BTreeSet<&str> =
        first.trim().trim_matches(['[', ']']).split(", ").collect();
    assert!(drawn.len() > 1, "{first}");
}

#[test]
fn an_evaluation_reports_the_seed_it_drew_and_repeats_under_it() {
    let program = "import random\nrange(0, 6) | map(_ => random_int(1, 1000))";
    let (value, reported) = hql_with(&["--report-level", "info", "eval", program], "");
    let seed = reported
        .split("the evaluation seed is ")
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .unwrap_or_else(|| panic!("no seed reported: {reported}"));
    assert_eq!(hql(&["--seed", seed, "eval", program]), value);
    // Under `--seed`, and where nothing is drawn, nothing is reported.
    let (_, quiet) = hql_with(
        &["--report-level", "info", "--seed", "1", "eval", program],
        "",
    );
    assert!(!quiet.contains("evaluation seed"), "{quiet}");
    let (_, pure) = hql_with(
        &[
            "--report-level",
            "info",
            "eval",
            "import random\n[1, 2] | shuffle(seed = 1)",
        ],
        "",
    );
    assert!(!pure.contains("evaluation seed"), "{pure}");
}

#[test]
fn uuid_gives_a_version_7_in_draw_order_and_a_version_5() {
    // RFC 9562's version 5 vector: "www.example.com" in the DNS namespace.
    assert_eq!(
        shown("uuid(\"www.example.com\", \"dns\")"),
        "2ed6657d-e927-568b-95e1-2665a8aea6a2"
    );
    assert_eq!(
        shown("uuid(\"www.example.com\", \"6ba7b810-9dad-11d1-80b4-00c04fd430c8\")"),
        "2ed6657d-e927-568b-95e1-2665a8aea6a2"
    );
    assert!(refused("uuid(\"a\", \"nowhere\")").contains("\"nowhere\""));
    let drawn = hql(&[
        "--now",
        "2026-10-24T07:30:00Z",
        "--seed",
        "1",
        "eval",
        "import random\nrange(0, 50) | map(_ => uuid())",
    ]);
    let ids: Vec<&str> = drawn
        .trim()
        .trim_matches(['[', ']'])
        .split(", ")
        .map(|id| id.trim_matches('"'))
        .collect();
    assert_eq!(ids.len(), 50);
    // 2026-10-24T07:30:00Z is 1 792 827 000 000 Unix milliseconds.
    let millis = format!("{:012x}", 1_792_827_000_000_u64);
    for id in &ids {
        assert_eq!(id.len(), 36, "{id}");
        assert_eq!(*id, id.to_lowercase());
        let hex: String = id.chars().filter(|c| *c != '-').collect();
        assert_eq!(&hex[..12], millis, "{id}");
        assert_eq!(&hex[12..13], "7", "{id}");
        assert!(matches!(&hex[16..17], "8" | "9" | "a" | "b"), "{id}");
    }
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "{drawn}");
}

#[test]
fn a_stream_is_sampled_holding_n() {
    let lines: String = (1..=10_000).map(|n| format!("{n}\n")).collect();
    let program = "import random\ntap(stdin, Lines) | sample(5)";
    let (first, _) = hql_with(&["--seed", "4", "eval", program], &lines);
    let (again, _) = hql_with(&["--seed", "4", "eval", program], &lines);
    assert_eq!(first, again);
    assert_eq!(
        first,
        "[\"1428\", \"2056\", \"4135\", \"8198\", \"8742\"]\n"
    );
    let drawn: Vec<u32> = first
        .trim()
        .trim_matches(['[', ']'])
        .split(", ")
        .map(|n| n.trim_matches('"').parse().unwrap())
        .collect();
    assert_eq!(drawn.len(), 5, "{first}");
    assert!(drawn.windows(2).all(|pair| pair[0] < pair[1]), "{first}");
    let (short, _) = hql_with(&["eval", program], "1\n2\n3\n");
    assert_eq!(short, "[\"1\", \"2\", \"3\"]\n");
}

#[test]
fn a_seeded_form_is_pure_and_an_unseeded_one_effectful() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("p.hql");
    std::fs::write(
        &path,
        "import random\n\
         fn pick(xs: List<Int>) : List<Int> = xs | sample(2)\n\
         fn fixed(xs: List<Int>) : List<Int> = xs | sample(2, seed = 3)\n\
         fn named(text: String) : String = uuid(text, \"dns\")\n\
         fn fresh() : String = uuid()\n\
         fixed([1, 2, 3])\n",
    )
    .unwrap();
    let report = hql(&["check", path.to_str().unwrap()]);
    assert!(
        report.contains("pick is Effectful: it uses `sample`"),
        "{report}"
    );
    assert!(
        report.contains("fresh is Effectful: it uses `uuid`"),
        "{report}"
    );
    assert!(!report.contains("fixed is Effectful"), "{report}");
    assert!(!report.contains("named is Effectful"), "{report}");
}

#[test]
fn random_is_imported_not_in_the_prelude() {
    let refused = hql::eval("[1, 2] | shuffle(seed = 1)").unwrap_err();
    assert!(
        refused.message.contains("import random"),
        "{}",
        refused.message
    );
}
