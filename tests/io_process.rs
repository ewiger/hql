//! The module `io.process` (HQL-0064, the companion note on child
//! processes): the `commands` grant and `[command_output] invalid_utf8` of a
//! project's `.hql/config.toml`, and the environment a child inherits.
//!
//! Every case runs the binary with its environment cleared and the variables
//! it names set, `PATH` among them so a program is found. The children are
//! Unix programs, so the cases run on Unix.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The configuration file of a directory that is a project.
fn project_config(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join(".hmd")).unwrap();
    std::fs::write(dir.join(".hmd/config.toml"), "wiki = \".\"\n").unwrap();
    std::fs::create_dir_all(dir.join(".hql")).unwrap();
    dir.join(".hql/config.toml")
}

fn path() -> String {
    std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned())
}

/// `hql` run in `directory` with only `PATH` and `variables` set.
fn hql_in(directory: &Path, variables: &[(&str, &str)], arguments: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hql"));
    command
        .current_dir(directory)
        .env_clear()
        .env("PATH", path())
        .env("HQL_HOME", directory.join(".hql-home"))
        .env("HQL_SYSTEM_CONFIG", directory.join("no-system.toml"))
        .args(arguments);
    for (name, value) in variables {
        command.env(name, value);
    }
    command.output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// `program` run in a project whose `.hql/config.toml` is `config`.
fn in_project(config: &str, variables: &[(&str, &str)], program: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(project_config(directory.path()), config).unwrap();
    let file = directory.path().join("p.hql");
    std::fs::write(&file, format!("from io.process import *\n{program}\n")).unwrap();
    hql_in(
        directory.path(),
        variables,
        &["run", file.to_str().unwrap()],
    )
}

/// `program` run on its own, outside any project.
fn standalone(variables: &[(&str, &str)], program: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let source = format!("from io.process import *\n{program}");
    hql_in(
        directory.path(),
        variables,
        &["--bare", "eval", source.as_str()],
    )
}

const RAN: &str =
    r#"match command("printf", ["wren"]).run() { ok(o) => o.stdout, err(e) => e.reason }"#;

#[test]
fn a_run_on_its_own_grants_every_program() {
    let output = standalone(&[], RAN);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"wren\"");
}

#[test]
fn a_project_without_commands_grants_every_program() {
    let output = in_project("", &[], RAN);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"wren\"");
}

#[test]
fn the_commands_grant_starts_a_listed_program() {
    let output = in_project("commands = [\"printf\"]\n", &[], RAN);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"wren\"");
}

#[test]
fn the_commands_grant_refuses_a_program_it_does_not_list() {
    let output = in_project(
        "commands = [\"printf\"]\n",
        &[],
        r#"command("sh", ["-c", "echo started"]).run()"#,
    );
    assert!(!output.status.success());
    assert!(!stdout(&output).contains("started"));
    assert!(
        stderr(&output).contains("not in the project's `commands` grant"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_refused_program_stops_every_operation() {
    let output = in_project(
        "commands = []\n",
        &[],
        r#"source(command("printf", ["wren"]), Lines)"#,
    );
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("`commands` grant"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_batch_file_needs_cmd_in_the_grant() {
    let output = in_project(
        "commands = [\"build.bat\"]\n",
        &[],
        r#"command("build.bat").run()"#,
    );
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("run by `cmd`"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn commands_that_is_not_a_list_is_refused() {
    let output = in_project("commands = \"printf\"\n", &[], RAN);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("`commands` is a list of program names"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn text_that_is_not_utf8_fails_by_default() {
    let output = in_project(
        "",
        &[],
        r#"match command("printf", ["\\377wren"]).run() { ok(o) => o.stdout, err(e) => e.reason }"#,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"invalid-text\"");
}

#[test]
fn replace_gives_the_replacement_character_under_run() {
    let output = in_project(
        "[command_output]\ninvalid_utf8 = \"replace\"\n",
        &[],
        r#"match command("printf", ["\\377wren"]).run() { ok(o) => o.stdout, err(e) => e.reason }"#,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"\u{FFFD}wren\"");
}

#[test]
fn replace_gives_the_replacement_character_under_a_read() {
    let output = in_project(
        "[command_output]\ninvalid_utf8 = \"replace\"\n",
        &[],
        r#"source(command("printf", ["\\377wren\\n"]), Lines)"#,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "[\"\u{FFFD}wren\"]");
}

#[test]
fn an_unknown_invalid_utf8_setting_is_refused() {
    let output = in_project("[command_output]\ninvalid_utf8 = \"skip\"\n", &[], RAN);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("\"fail\" or \"replace\""),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_child_inherits_only_the_granted_variables() {
    let output = in_project(
        "env = [\"APP_*\"]\n",
        &[("APP_TOKEN", "t0k"), ("OTHER", "o")],
        r#"match command("sh", ["-c", "printf '%s-%s' \"$APP_TOKEN\" \"$OTHER\""]).run() { ok(o) => o.stdout, err(e) => e.reason }"#,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"t0k-\"");
}

#[test]
fn env_adds_variables_to_the_child() {
    let output = standalone(
        &[],
        r#"match command("sh", ["-c", "printf %s \"$BIRD\""], env = {BIRD: "wren"}).run() { ok(o) => o.stdout, err(e) => e.reason }"#,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "\"wren\"");
}

#[test]
fn dir_is_the_childs_working_directory() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("nest")).unwrap();
    let file = directory.path().join("p.hql");
    std::fs::write(
        &file,
        "from io.process import *\n\
         match command(\"pwd\", dir = \"nest\").run() { ok(o) => o.stdout | trim, err(e) => e.reason }\n",
    )
    .unwrap();
    let output = hql_in(
        directory.path(),
        &[],
        &["--bare", "run", file.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).trim_end().ends_with("/nest\""),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_failed_read_states_the_reason_and_the_last_lines_of_standard_error() {
    let output = standalone(
        &[],
        r#"source(command("sh", ["-c", "echo the disk is full >&2; exit 4"]), Lines)"#,
    );
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("failed: exited, status 4\n"), "{err}");
    assert!(
        err.contains("= caused by: its standard error ended with:"),
        "{err}"
    );
    assert!(err.contains("the disk is full"), "{err}");
}

#[test]
fn the_standard_error_of_a_failed_child_is_the_cause_in_the_channel() {
    let output = standalone(
        &[],
        r#"source(command("sh", ["-c", "echo the disk is full >&2; exit 4"]), Lines)"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let source = "from io.process import *\n\
                  source(command(\"sh\", [\"-c\", \"echo the disk is full >&2; exit 4\"]), Lines)";
    let enveloped = hql_in(
        directory.path(),
        &[],
        &["--bare", "--envelope", "json", "eval", source],
    );
    assert!(!output.status.success());
    let envelope: serde_json::Value = serde_json::from_slice(&enveloped.stdout).unwrap();
    let diagnostic = &envelope["diagnostics"][0];
    assert_eq!(
        diagnostic["message"],
        "command `sh`: failed: exited, status 4"
    );
    assert_eq!(
        diagnostic["causes"],
        serde_json::json!(["its standard error ended with:\nthe disk is full"])
    );
}

#[test]
fn a_failed_child_read_as_json_is_its_failure_and_not_a_decoding_one() {
    let output = standalone(&[], r#"source(command("sh", ["-c", "exit 2"]), Json)"#);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(
        err.contains("command `sh`: failed: exited, status 2"),
        "{err}"
    );
    assert!(!err.contains("not JSON"), "{err}");
}

#[test]
fn a_failed_run_as_the_result_exits_one() {
    let output = standalone(&[], r#"command("sh", ["-c", "exit 3"]).run()"#);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("err(CommandFailure("),
        "{}",
        stdout(&output)
    );
}

/// `hql` running a child that records its process group and starts a
/// process of its own, ended by `signal` once the group is recorded.
fn ended_by(signal: &str) -> (std::process::ExitStatus, bool) {
    use std::os::unix::process::ExitStatusExt;
    let directory = tempfile::tempdir().unwrap();
    let recorded = directory.path().join("group");
    let source = format!(
        "from io.process import *\n\
         source(command(\"sh\", [\"-c\", \"echo $$ > {}; sleep 30 & sleep 30\"]), Lines)",
        recorded.display()
    );
    let mut hql = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(directory.path())
        .env_clear()
        .env("PATH", path())
        .env("HQL_HOME", directory.path().join(".hql-home"))
        .args(["--bare", "eval", source.as_str()])
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let group = (0..500)
        .find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            std::fs::read_to_string(&recorded)
                .ok()
                .filter(|text| text.ends_with('\n'))
        })
        .expect("the child recorded its group");
    // Let `sh` start the background `sleep` before the signal.
    std::thread::sleep(std::time::Duration::from_millis(200));
    Command::new("kill")
        .args([format!("-{signal}"), hql.id().to_string()])
        .status()
        .unwrap();
    let status = hql.wait().unwrap();
    assert!(status.signal().is_some() || status.code().is_some());
    let gone = (0..100).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(10));
        !Command::new("kill")
            .args(["-0", &format!("-{}", group.trim())])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    });
    (status, gone)
}

#[test]
fn a_signal_that_ends_hql_ends_its_children_and_what_they_started() {
    use std::os::unix::process::ExitStatusExt;
    for (name, number) in [("TERM", 15), ("INT", 2), ("HUP", 1)] {
        let (status, gone) = ended_by(name);
        assert_eq!(status.signal(), Some(number), "{name}: {status:?}");
        assert!(gone, "{name}: a process of the child's group survived");
    }
}
