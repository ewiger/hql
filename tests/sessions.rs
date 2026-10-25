//! Replay the REPL and command-line transcripts under `examples/`.
//!
//! A `.session` file opens with the corpus header, then a transcript: in a
//! `repl` session a line opening `hql> ` is an input line, and in a `cli`
//! session a line opening `$ ` is a command run from the repository root.
//! Every other line is expected output, standard output and standard error
//! in the order written; `{version}` stands for the binary's version.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Session {
    path: PathBuf,
    metadata: BTreeMap<String, String>,
    transcript: String,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sessions() -> Vec<Session> {
    fn visit(directory: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, found);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "session")
            {
                found.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    visit(&root().join("examples"), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            let (header, transcript) = text
                .split_once("\n\n")
                .unwrap_or_else(|| panic!("{}: missing header", path.display()));
            let metadata = header
                .lines()
                .map(|line| {
                    let line = line.strip_prefix("// ").expect("a header line");
                    let (key, value) = line.split_once(": ").expect("a header field");
                    (key.to_owned(), value.to_owned())
                })
                .collect();
            Session {
                path,
                metadata,
                transcript: transcript.replace("{version}", env!("CARGO_PKG_VERSION")),
            }
        })
        .collect()
}

/// Run the binary with its standard error joined to its standard output,
/// so the two are read in the order written.
fn run(arguments: &[String], input: &str) -> (String, i32) {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("\"$0\" \"$@\" 2>&1")
        .arg(env!("CARGO_BIN_EXE_hql"))
        .args(arguments)
        .current_dir(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

/// What a session prints, and what its transcript says it prints.
fn replay(session: &Session) -> (String, String) {
    let modules: Vec<String> = session
        .metadata
        .get("modules")
        .map(|modules| vec!["--modules".to_owned(), format!("examples/{modules}")])
        .unwrap_or_default();
    match session.metadata["session"].as_str() {
        "repl" => {
            let mut input = String::new();
            let mut expected = String::new();
            for line in session.transcript.lines() {
                if line == "hql>" {
                    input.push('\n');
                } else if let Some(written) = line.strip_prefix("hql> ") {
                    input.push_str(written);
                    input.push('\n');
                } else {
                    expected.push_str(line);
                    expected.push('\n');
                }
            }
            let mut arguments = modules;
            arguments.push("repl".to_owned());
            (run(&arguments, &input).0, expected)
        }
        "cli" => {
            let mut printed = String::new();
            let mut expected = String::new();
            let mut status = 0;
            for line in session.transcript.lines() {
                if let Some(command) = line.strip_prefix("$ ") {
                    let words: Vec<String> =
                        command.split_whitespace().map(str::to_owned).collect();
                    assert_eq!(words[0], "hql", "{}", session.path.display());
                    let (output, code) = run(&words[1..], "");
                    printed.push_str(&output);
                    status = code;
                } else {
                    expected.push_str(line);
                    expected.push('\n');
                }
            }
            let declared: i32 = session.metadata["exit"].parse().unwrap();
            if status != declared {
                printed.push_str(&format!("[exit {status}]\n"));
            }
            (printed, expected)
        }
        other => panic!("{}: unknown session kind {other}", session.path.display()),
    }
}

#[test]
fn sessions_are_described_and_indexed() {
    let sessions = sessions();
    assert!(!sessions.is_empty());
    for session in &sessions {
        let m = &session.metadata;
        for field in [
            "status",
            "feature",
            "implementation",
            "environment",
            "session",
        ] {
            assert!(
                m.contains_key(field),
                "{}: missing {field}",
                session.path.display()
            );
        }
        match m["status"].as_str() {
            "proposed" => {
                assert_eq!(m["implementation"], "pending", "{}", session.path.display());
                assert!(
                    root().join(&m["proposal"]).is_file(),
                    "{}",
                    session.path.display()
                );
            }
            "valid-now" => {
                assert_eq!(
                    m["implementation"],
                    "implemented",
                    "{}",
                    session.path.display()
                );
                assert!(!m.contains_key("proposal"), "{}", session.path.display());
            }
            other => panic!("{}: unknown status {other}", session.path.display()),
        }
        if m["session"] == "cli" {
            assert!(
                m.contains_key("exit"),
                "{}: missing exit",
                session.path.display()
            );
        }
        let readme = session
            .path
            .ancestors()
            .skip(1)
            .find(|directory| directory.join("README.md").is_file())
            .unwrap();
        let relative = session
            .path
            .strip_prefix(readme)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let index = std::fs::read_to_string(readme.join("README.md")).unwrap();
        assert!(
            index.contains(&format!("]({relative})")),
            "{}: missing README link",
            session.path.display()
        );
    }
}

#[test]
fn valid_sessions_print_their_transcripts() {
    for session in sessions() {
        if session.metadata["status"] != "valid-now" {
            continue;
        }
        let (printed, expected) = replay(&session);
        assert_eq!(printed, expected, "{}", session.path.display());
    }
}

/// A proposed session is a design input: the day it passes, this test names
/// it so that its header is promoted.
#[test]
fn proposed_sessions_do_not_pass_yet() {
    for session in sessions() {
        if session.metadata["status"] != "proposed" {
            continue;
        }
        let (printed, expected) = replay(&session);
        assert_ne!(
            printed,
            expected,
            "{}: passes now; promote it to valid-now",
            session.path.display()
        );
    }
}
