//! The module `io.process` on Windows (HQL-0064, the companion note on
//! child processes, the section "Windows"): a child runs in a Job Object,
//! so `timeout_ms` and a stream not read to its end end it.
#![cfg(windows)]

use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn hql(source: &str) -> (Output, Duration) {
    let directory = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .env("HQL_HOME", directory.path().join(".hql-home"))
        .args([
            "--bare",
            "eval",
            &format!("from io.process import *\n{source}"),
        ])
        .output()
        .unwrap();
    (output, started.elapsed())
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

#[test]
fn a_child_running_past_its_timeout_is_ended() {
    let (output, took) = hql(
        r#"match command("ping", ["-n", "30", "127.0.0.1"], timeout_ms = 300).run() { ok(o) => "ran", err(e) => e.reason }"#,
    );
    assert_eq!(stdout(&output), "\"timed-out\"", "{output:?}");
    assert!(took < Duration::from_secs(10), "took {took:?}");
}

#[test]
fn a_stream_not_read_to_its_end_ends_the_child() {
    let (output, took) =
        hql(r#"tap(command("ping", ["-n", "30", "127.0.0.1"]), Lines) | take(2) | size"#);
    assert_eq!(stdout(&output), "2", "{output:?}");
    assert!(took < Duration::from_secs(10), "took {took:?}");
}
