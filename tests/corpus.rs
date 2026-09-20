//! Check runnable examples against their declared types, results, and failures.

use std::{collections::BTreeMap, fs, path::Path};

use hql::{check, check_in, diagnostics::Diagnostic, eval, reporting::Mode, run, vault};

struct Case {
    path: String,
    metadata: BTreeMap<String, String>,
    source: String,
}

impl Case {
    fn vault(&self) -> vault::Vault {
        match self.metadata["environment"].as_str() {
            "pure" => vault::Vault::empty(),
            "knowledge-v1" => {
                let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples")
                    .join(&self.metadata["vault"]);
                vault::load(&path).unwrap_or_else(|e| panic!("{}: {e}", self.path))
            }
            other => panic!("{}: unsupported environment {other}", self.path),
        }
    }
}

fn load_cases() -> Vec<Case> {
    fn visit(root: &Path, directory: &Path, cases: &mut Vec<Case>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path == root.join("fixtures") {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, cases);
                continue;
            }
            // Preserve the original header-free CLI smoke fixture.
            if path == root.join("answer.hql") {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("hql") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            let (header, source) = text
                .split_once("\n\n")
                .unwrap_or_else(|| panic!("{}: missing corpus header", path.display()));
            let mut metadata = BTreeMap::new();
            for line in header.lines() {
                let line = line.strip_prefix("// ").expect("invalid HQL metadata line");
                let (key, value) = line.split_once(": ").expect("invalid metadata field");
                assert!(!value.is_empty(), "{}: empty {key}", path.display());
                assert!(metadata.insert(key.to_owned(), value.to_owned()).is_none());
            }
            cases.push(Case {
                path: path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                metadata,
                source: source.to_owned(),
            });
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut cases = Vec::new();
    visit(&root, &root, &mut cases);
    cases.sort_by(|a, b| a.path.cmp(&b.path));
    cases
}

#[test]
fn corpus_metadata_and_index_cover_every_case() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let cases = load_cases();
    assert!(!cases.is_empty());
    for case in cases {
        let m = &case.metadata;
        for field in ["status", "feature", "implementation", "environment"] {
            assert!(m.contains_key(field), "{}: missing {field}", case.path);
        }
        let status = m["status"].as_str();
        assert!(
            matches!(status, "valid-now" | "invalid" | "proposed"),
            "{}",
            case.path
        );
        if status == "proposed" {
            // Written ahead of the implementation: the header pins the type
            // and result the case is meant to produce, and `proposal` names
            // the record that specifies them.
            assert_eq!(m["implementation"], "pending", "{}", case.path);
            let proposal = Path::new(env!("CARGO_MANIFEST_DIR")).join(&m["proposal"]);
            assert!(
                proposal.is_file(),
                "{}: no proposal at {}",
                case.path,
                m["proposal"]
            );
        } else {
            assert_eq!(m["implementation"], "implemented", "{}", case.path);
            assert!(!m.contains_key("proposal"), "{}", case.path);
        }
        match m["environment"].as_str() {
            "pure" => assert!(!m.contains_key("vault"), "{}", case.path),
            "knowledge-v1" => assert!(root.join(&m["vault"]).is_dir(), "{}", case.path),
            other => panic!("{}: unsupported environment {other}", case.path),
        }
        if status == "valid-now" || status == "proposed" {
            assert!(m.contains_key("expected-type"), "{}", case.path);
            assert!(
                m.contains_key("expected") ^ m.contains_key("expected-json"),
                "{}: declare one expected result",
                case.path
            );
            if let Some(json) = m.get("expected-json") {
                serde_json::from_str::<serde_json::Value>(json).expect("a JSON expectation");
            }
        }
        if status == "invalid" {
            assert!(m.contains_key("error") && m.contains_key("stage"));
            // `eval` exposes the precise Diagnostic for pure failure examples.
            assert_eq!(m["environment"], "pure");
            if m["stage"] == "evaluate" {
                assert!(m.contains_key("expected-type"), "{}", case.path);
            }
        }
        let path = root.join(&case.path);
        let directory = path
            .parent()
            .unwrap()
            .ancestors()
            .find(|directory| directory.join("README.md").is_file())
            .expect("an example README");
        let index = fs::read_to_string(directory.join("README.md")).unwrap();
        let relative = path
            .strip_prefix(directory)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(
            index.contains(&format!("]({relative})")),
            "{}: missing README link",
            case.path
        );
    }
}

#[test]
fn valid_now_cases_check_and_evaluate() {
    let mut count = 0;
    for case in load_cases() {
        if case.metadata["status"] != "valid-now" {
            continue;
        }
        count += 1;
        let vault = case.vault();
        let ty = check_in(&case.source, &vault).unwrap_or_else(|e| panic!("{}: {e}", case.path));
        let outcome = run(&case.source, &vault, Mode::Strict);
        assert!(
            outcome.reports.is_empty(),
            "{}: {:?}",
            case.path,
            outcome.reports
        );
        assert_eq!(outcome.inferred.as_ref(), Some(&ty), "{}", case.path);
        let value = outcome
            .value
            .expect("a successful example produces a value");
        assert_eq!(
            ty.to_string(),
            case.metadata["expected-type"],
            "{}",
            case.path
        );
        if let Some(expected) = case.metadata.get("expected-json") {
            assert_eq!(
                value.to_json(),
                serde_json::from_str::<serde_json::Value>(expected).unwrap(),
                "{}",
                case.path
            );
        } else {
            assert_eq!(
                value.to_string(),
                case.metadata["expected"],
                "{}",
                case.path
            );
        }
    }
    assert!(count > 0);
}

/// A proposed case is a design input, not a regression test. It stays
/// proposed only while the implementation cannot check it; the day it checks,
/// this test names it so that its header is promoted rather than left claiming
/// a step does not exist.
#[test]
fn proposed_cases_do_not_check_yet() {
    for case in load_cases() {
        if case.metadata["status"] != "proposed" {
            continue;
        }
        let vault = case.vault();
        assert!(
            check_in(&case.source, &vault).is_err(),
            "{}: checks now; promote it to valid-now",
            case.path
        );
    }
}

#[test]
fn implemented_invalid_cases_fail_at_the_declared_stage() {
    let mut count = 0;
    for case in load_cases() {
        let m = &case.metadata;
        if m["status"] != "invalid" || m["implementation"] != "implemented" {
            continue;
        }
        count += 1;
        let error = match m["stage"].as_str() {
            "parse" | "typecheck" => {
                let error = check(&case.source).expect_err(&case.path);
                assert_eq!(eval(&case.source), Err(error.clone()), "{}", case.path);
                error
            }
            "evaluate" => {
                assert_eq!(check(&case.source).unwrap().to_string(), m["expected-type"]);
                eval(&case.source).expect_err(&case.path)
            }
            other => panic!("no implemented corpus runner for {other}"),
        };
        let (name, stage) = match error {
            Diagnostic::Syntax { .. } => ("SyntaxError", "parse"),
            Diagnostic::Type { .. } => ("TypeMismatch", "typecheck"),
            Diagnostic::Overflow { .. } => ("IntegerOverflow", "evaluate"),
            Diagnostic::Name { .. } => ("NameError", "typecheck"),
            Diagnostic::Runtime { .. } => ("RuntimeError", "evaluate"),
        };
        assert_eq!(name, m["error"], "{}", case.path);
        assert_eq!(stage, m["stage"], "{}", case.path);
    }
    assert!(count > 0);
}
