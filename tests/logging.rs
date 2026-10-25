//! Loggers through the real process: events written as `sink` would, to a
//! file's end, a channel, or a cell's logs. See
//! doc/proposals/HQL-0021/README.md.

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn hql_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn refused(output: &Output, says: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains(says), "{stderr}");
}

#[test]
fn a_logger_is_imported() {
    let dir = tempdir().unwrap();
    refused(
        &hql_in(dir.path(), &[r#"Logger(stderr, Text, name = "x")"#]),
        "write `import logging`",
    );
}

#[test]
fn a_log_file_is_appended_to_one_json_document_per_event() {
    let dir = tempdir().unwrap();
    let program = "import logging\n\
                   log = Logger(file(\"app.log\").end, Json, name = \"a\", level = Debug)\n\
                   log | debug(\"one\")\n\
                   info(log, \"two\", {n: 2})";
    for _ in 0..2 {
        assert_eq!(ok(&hql_in(dir.path(), &[program])), "");
    }
    let line = "{\"data\":{},\"level\":\"Debug\",\"logger\":\"a\",\"message\":\"one\"}\n\
                {\"data\":{\"n\":2},\"level\":\"Info\",\"logger\":\"a\",\"message\":\"two\"}\n";
    assert_eq!(
        fs::read_to_string(dir.path().join("app.log")).unwrap(),
        line.repeat(2)
    );
}

#[test]
fn a_threshold_drops_less_severe_events() {
    let dir = tempdir().unwrap();
    let program = "import logging\n\
                   log = Logger(file(\"q.log\"), Text, name = \"q\", level = Warn)\n\
                   log | info(\"dropped\")\n\
                   log | error(\"kept\")";
    ok(&hql_in(dir.path(), &[program]));
    assert_eq!(
        fs::read_to_string(dir.path().join("q.log")).unwrap(),
        "ERROR q kept\n"
    );
}

#[test]
fn logs_in_a_channel_are_data_a_program_queries() {
    let dir = tempdir().unwrap();
    let program = "import logging\n\
                   c = Channel<LogEvent>(\"logs\")\n\
                   log = Logger(c, name = \"x\")\n\
                   log | info(\"a\")\n\
                   log | error(\"b\")\n\
                   source(c) | filter(e => e.level | is(Error)) | map(e => e.message)";
    assert_eq!(ok(&hql_in(dir.path(), &[program])), "[\"b\"]\n");
    refused(
        &hql_in(
            dir.path(),
            &["import logging\nc = Channel<Int>(\"n\")\nLogger(c, name = \"x\")"],
        ),
        "writes LogEvents",
    );
}

#[test]
fn a_cell_logs_to_its_own_channel_and_saves_only_its_result() {
    let dir = tempdir().unwrap();
    // A document renders only in a vault.
    std::fs::write(dir.path().join("hql.toml"), "").unwrap();
    fs::write(
        dir.path().join("doc.md"),
        "```hql\nimport logging\nlog = Logger(cell.logs, name = \"r\")\n\
         log | info(\"hi\")\n[1, 2] | size\n```\n",
    )
    .unwrap();
    let rendered = ok(&hql_in(dir.path(), &["render", "doc.md"]));
    assert_eq!(rendered.trim(), "2");
    fs::write(
        dir.path().join("loud.md"),
        "```hql\nimport logging\nlog = Logger(stderr, Text, name = \"r\")\n1\n```\n",
    )
    .unwrap();
    refused(
        &hql_in(dir.path(), &["render", "loud.md"]),
        "log to `cell.logs`",
    );
}
