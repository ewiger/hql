//! Exercise the actual CLI process, including file and failure behavior.

use std::{
    fs,
    process::{Command, Output},
};
use tempfile::tempdir;

fn hql(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn eval_prints_values() {
    for (expression, expected) in [
        ("40 + 2", "42\n"),
        ("true", "true\n"),
        ("-1 + 2", "1\n"),
        ("0.5 + 0.25", "0.75\n"),
        ("1.0 + 2.0", "3.0\n"),
    ] {
        let output = hql(&["eval", expression]);
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn expression_failures_use_stderr_and_exit_one() {
    for (source, message) in [
        ("1 +", "syntax error"),
        ("true + 1", "type error"),
        ("9223372036854775807 + 1", "numeric overflow"),
        ("1 + 2.0", "type error"),
    ] {
        let output = hql(&["eval", source]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains(message));
    }
}

#[test]
fn check_reads_files_without_evaluation_and_reports_errors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("example.hql");
    let name = path.to_str().unwrap();
    fs::write(&path, "9223372036854775807 + 1\n").unwrap();
    let output = hql(&["check", name]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Int\n");
    for bytes in [b"true + 1".as_slice(), b"1 +", b"\xff"] {
        fs::write(&path, bytes).unwrap();
        let output = hql(&["check", name]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains(name));
    }
    let missing = dir.path().join("missing.hql");
    assert_eq!(
        hql(&["check", missing.to_str().unwrap()]).status.code(),
        Some(1)
    );
}

#[test]
fn cli_surface_is_small_and_identifies_hql() {
    let help = hql(&["--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("Hyper Query Language")
    );
    assert_eq!(hql(&["--version"]).stdout, b"hql 0.1.0\n");
    for args in [vec![], vec!["eval"], vec!["check"], vec!["unknown"]] {
        assert_eq!(hql(&args).status.code(), Some(2));
    }
}
