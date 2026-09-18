//! Exercise the actual CLI process: I/O, exit codes, formats and modes.

use std::io::Write;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const VAULT: &str = "tests/fixtures/vault";

fn hql(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(args)
        .output()
        .unwrap()
}

fn piped(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn eval_prints_values() {
    for (expression, expected) in [
        ("40 + 2", "42\n"),
        ("true", "true\n"),
        ("-1 + 2", "1\n"),
        ("0.5 + 0.25", "0.75\n"),
        ("1.0 + 2.0", "3.0\n"),
        ("answer = 40 + 2\nanswer", "42\n"),
    ] {
        let output = hql(&["eval", expression]);
        assert!(output.status.success(), "{expression}: {}", stderr(&output));
        assert_eq!(stdout(&output), expected, "{expression}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn expression_failures_use_stderr_and_exit_one() {
    for (source, message) in [
        ("1 +", "syntax"),
        ("true + 1", "addition requires"),
        ("9223372036854775807 + 1", "numeric overflow"),
        ("1 + 2.0", "addition requires"),
        ("nosuchname", "is not bound"),
    ] {
        let output = hql(&["eval", source]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(output.stdout.is_empty(), "{source}");
        assert!(
            stderr(&output).contains(message),
            "{source}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn a_diagnostic_points_at_a_line_a_column_and_the_source() {
    let output = hql(&["eval", "1 +\n  true + 1"]);
    let reported = stderr(&output);
    assert!(reported.contains("<expression>:2:3"), "{reported}");
    assert!(reported.contains("^^^^"), "{reported}");
    assert!(reported.contains("  true + 1"), "{reported}");
}

#[test]
fn check_reads_files_without_evaluation_and_reports_errors() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("example.hql");
    let name = path.to_str().unwrap();
    std::fs::write(&path, "9223372036854775807 + 1\n").unwrap();
    let output = hql(&["check", name]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Int\n");

    for bytes in [b"true + 1".as_slice(), b"1 +", b"\xff"] {
        std::fs::write(&path, bytes).unwrap();
        let output = hql(&["check", name]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
    let missing = directory.path().join("missing.hql");
    let output = hql(&["check", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("missing.hql"));
}

#[test]
fn run_evaluates_a_file_and_dash_reads_standard_input() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("program.hql");
    std::fs::write(&path, "a = 2\nb = 40\na + b\n").unwrap();
    let output = hql(&["run", path.to_str().unwrap()]);
    assert_eq!(stdout(&output), "42\n");

    assert_eq!(stdout(&piped(&["run", "-"], "1 + 1\n")), "2\n");
    assert_eq!(stdout(&piped(&["check", "-"], "1 + 1\n")), "Int\n");
}

#[test]
fn json_carries_the_type_the_value_and_the_queue() {
    let output = hql(&["--format", "json", "eval", "40 + 2"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(parsed["type"], "Int");
    assert_eq!(parsed["value"], 42);
    assert_eq!(parsed["reports"].as_array().unwrap().len(), 0);

    let output = hql(&["--format", "json", "eval", "nosuchname"]);
    assert_eq!(output.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(parsed["value"].is_null());
    let reports = parsed["reports"].as_array().unwrap();
    assert_eq!(reports[0]["severity"], "error");
    assert_eq!(reports[0]["stage"], "name");
}

#[test]
fn the_vault_is_supplied_from_outside_and_its_absence_is_said_out_loud() {
    let output = hql(&["eval", "cards | count"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("--vault"), "{}", stderr(&output));

    let output = hql(&["--vault", VAULT, "eval", "cards | count"]);
    assert_eq!(stdout(&output), "6\n");
}

#[test]
fn the_whole_pipeline_runs_from_the_command_line() {
    let output = hql(&[
        "--vault",
        VAULT,
        "eval",
        "import semantic\ncards\n| semantic(\"bearer token authorization\")\n| take(2)\n| expand(depth = 1)\n| table",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let table = stdout(&output);
    assert!(table.contains("retrieved"), "{table}");
    assert!(table.contains("expanded from"), "{table}");
}

#[test]
fn collect_mode_reports_every_error_and_strict_reports_the_first() {
    let source = "a = nosuchthing\nb = alsomissing";
    let strict = hql(&["eval", source]);
    assert_eq!(stderr(&strict).matches("error:").count(), 1);

    let collected = hql(&["--report", "collect", "eval", source]);
    assert_eq!(stderr(&collected).matches("error:").count(), 2);
    assert!(
        stderr(&collected).contains("2 reports"),
        "{}",
        stderr(&collected)
    );

    let unknown = hql(&["--report", "loud", "eval", "1"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(stderr(&unknown).contains("unknown report mode"));
}

#[test]
fn the_environment_supplies_a_default_mode_and_the_flag_outranks_it() {
    let source = "a = nosuchthing\nb = alsomissing";
    let from_environment = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["eval", source])
        .env("HQL_REPORT", "collect")
        .output()
        .unwrap();
    assert_eq!(stderr(&from_environment).matches("error:").count(), 2);

    let overridden = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(["--report", "strict", "eval", source])
        .env("HQL_REPORT", "collect")
        .output()
        .unwrap();
    assert_eq!(stderr(&overridden).matches("error:").count(), 1);
}

#[test]
fn a_warning_does_not_change_the_exit_status() {
    let output = hql(&["--vault", VAULT, "eval", "[[nowhere]]"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "none\n");
    assert!(stderr(&output).contains("warning:"));
}

#[test]
fn render_transcludes_answers_and_write_is_idempotent() {
    let output = hql(&["--vault", VAULT, "render", "tests/fixtures/board.hmd"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("```hql#result"));

    let directory = tempdir().unwrap();
    let path = directory.path().join("board.hmd");
    std::fs::copy("tests/fixtures/board.hmd", &path).unwrap();
    let name = path.to_str().unwrap();

    let first = hql(&["--vault", VAULT, "render", name, "--write"]);
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(stdout(&first).contains("2 blocks rendered"));
    let once = std::fs::read_to_string(&path).unwrap();

    hql(&["--vault", VAULT, "render", name, "--write"]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), once);
}

#[test]
fn every_query_fixture_runs_against_the_fixture_vault() {
    // These are the files a person runs by hand to see the language work, so
    // they are pinned rather than left to rot.
    for (query, expected) in [
        ("todo.hql", "Session cookies"),
        ("nearest.hql", "expanded from bearer-tokens"),
        ("kinds.hql", "RelationCard"),
    ] {
        let path = format!("tests/fixtures/queries/{query}");
        let output = hql(&["--vault", VAULT, "run", &path]);
        assert!(output.status.success(), "{query}: {}", stderr(&output));
        assert!(
            stdout(&output).contains(expected),
            "{query}: {}",
            stdout(&output)
        );
    }
}

#[test]
fn the_repl_keeps_bindings_and_answers_type_questions() {
    let session = "a = 40\na + 2\n:type a + 2\n:quit\n";
    let output = piped(&["repl"], session);
    let printed = stdout(&output);
    assert!(printed.contains("42"), "{printed}");
    assert!(printed.contains("Int"), "{printed}");
    assert!(output.status.success());
}

#[test]
fn builtins_and_config_explain_the_surface_and_the_settings() {
    let listed = hql(&["builtins"]);
    assert!(listed.status.success());
    for name in ["semantic", "expand", "graph", "take", "typed"] {
        assert!(stdout(&listed).contains(name), "{name}");
    }

    let configured = hql(&["--vault", VAULT, "config"]);
    assert!(stdout(&configured).contains("strict"));
    assert!(stdout(&configured).contains("6 cards"));
}

#[test]
fn cli_surface_identifies_hql_and_usage_errors_exit_two() {
    let help = hql(&["--help"]);
    assert!(help.status.success());
    assert!(stdout(&help).contains("Hyper Query Language"));
    assert_eq!(hql(&["--version"]).stdout, b"hql 0.1.0\n");
    for args in [vec![], vec!["eval"], vec!["check"], vec!["unknown"]] {
        assert_eq!(hql(&args).status.code(), Some(2), "{args:?}");
    }
    let missing = hql(&["--vault", "tests/fixtures/nowhere", "eval", "1"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(stderr(&missing).contains("not a directory"));
}
