//! `Decimal` (HQL-0031): division to a hundred significant digits, and the
//! formats carrying a Decimal's digits and scale through a read and a write.

use std::io::Write;
use std::process::{Command, Stdio};

/// What `hql` writes for `program` with `input` on standard input.
fn piped(args: &[&str], input: &str) -> String {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .args(args)
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
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn shown(source: &str) -> String {
    hql::eval(source).unwrap().to_string()
}

const PAYMENT: &str = "type Payment {\n    item : String\n    amount : Decimal\n}\n";

#[test]
fn a_quotient_that_terminates_is_exact() {
    for (source, expected) in [
        ("1 / 8.0", "0.125"),
        ("1.00 / 4", "0.25"),
        ("10 / 4.0", "2.5"),
        ("-7.5 / 2.5", "-3"),
        ("0.000001 / 1024", "0.0000000009765625"),
    ] {
        assert_eq!(shown(source), expected, "{source}");
    }
}

#[test]
fn a_quotient_that_does_not_terminate_keeps_a_hundred_significant_digits() {
    assert_eq!(shown("1 / 3.0"), format!("0.{}", "3".repeat(100)));
    // Half to even on the digit past the hundredth, which is a 6.
    assert_eq!(shown("2 / 3.0"), format!("0.{}7", "6".repeat(99)));
    assert_eq!(shown("200 / 3.0"), format!("66.{}7", "6".repeat(97)));
    assert_eq!(shown("1 / 3.0 * 3"), format!("0.{}", "9".repeat(100)));
    assert_eq!(shown("round(2 / 3.0, 4)"), "0.6667");
    let seventh = shown("1 / 7.0");
    let digits = seventh.trim_start_matches("0.");
    assert_eq!(digits.len(), 100, "{seventh}");
    assert!(digits.starts_with("142857142857"), "{seventh}");
}

#[test]
fn division_by_zero_is_an_evaluation_error() {
    assert!(matches!(
        hql::eval("1.50 / 0"),
        Err(hql::diagnostics::Diagnostic::Runtime { .. })
    ));
    assert!(matches!(
        hql::eval("1.50 % 0.0"),
        Err(hql::diagnostics::Diagnostic::Runtime { .. })
    ));
}

#[test]
fn json_carries_the_digits_through_a_read_and_a_write() {
    let input = "{\"item\": \"bond\", \"amount\": 1234567890.123456789012345}\n\
                 {\"item\": \"tea\", \"amount\": 0.10}\n";
    let program = format!("{PAYMENT}source(stdin, Json) | Payment");
    assert_eq!(
        piped(&[&program, "--to", "json", "--compact"], input),
        "{\"amount\":1234567890.123456789012345,\"item\":\"bond\"}\n\
         {\"amount\":0.10,\"item\":\"tea\"}\n"
    );
    let total =
        format!("{PAYMENT}source(stdin, Json) | Payment | map(p => p.amount) | take(2) | sum");
    assert_eq!(
        piped(&[&total, "--to", "json"], input),
        "1234567890.223456789012345\n"
    );
    // Read as a Float, the same number is the nearest binary float.
    let float = "type Rough : Data = { amount : Float }\nsource(stdin, Json) | Rough | map(r => r.amount) | take(1)";
    assert_eq!(piped(&[float], input), "[1234567890.1234567]\n");
}

#[test]
fn yaml_carries_the_digits_through_a_read_and_a_write() {
    let input = "item: tea\namount: 0.10\n";
    let program = format!("{PAYMENT}source(stdin, Yaml) | Payment");
    assert_eq!(
        piped(&[&program, "--to", "yaml"], input),
        "---\namount: 0.10\nitem: tea\n"
    );
    let long = "item: bond\namount: 1234567890.123456789012345\n";
    assert_eq!(
        piped(&[&program, "--to", "json", "--compact"], long),
        "{\"amount\":1234567890.123456789012345,\"item\":\"bond\"}\n"
    );
}

#[test]
fn csv_carries_the_digits_through_a_read_and_a_write() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("prices.csv"),
        "item,amount\ntea,0.10\nbond,1234567890.123456789012345\n",
    )
    .unwrap();
    let program = format!(
        "{PAYMENT}rows : Seq<Payment> = source(file(\"prices.csv\"))\n\
         Table(rows | List, columns = [\"item\", \"amount\"])"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .args([program.as_str(), "--to", "csv"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "item,amount\r\ntea,0.10\r\nbond,1234567890.123456789012345\n"
    );
}

#[test]
fn a_decimal_converts_to_and_from_text_and_the_other_numbers() {
    assert_eq!(shown("\"12.50\".to_decimal()"), "12.50");
    assert_eq!(shown("\"1e400\".to_decimal().is_some()"), "true");
    assert_eq!(shown("\"twelve\".to_decimal()"), "none");
    assert_eq!(shown("decimal(0.1f)"), "0.1");
    assert_eq!(shown("float(0.1)"), "0.1");
    assert_eq!(shown("int(-2.75)"), "-2");
    assert_eq!(shown("[-2.50.abs(), 2.50.sign()]"), "[2.50, 1]");
}
