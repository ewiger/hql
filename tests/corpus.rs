//! Validate the design corpus and run only explicitly implemented cases.

use std::{collections::BTreeMap, fs, path::Path};

use hql::{check, diagnostics::Diagnostic, eval};

struct Case {
    path: String,
    metadata: BTreeMap<String, String>,
    source: String,
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
            let extension = path.extension().and_then(|s| s.to_str());
            if !matches!(extension, Some("hql" | "hmd")) {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            let (header, source) = if extension == Some("hql") {
                let (header, source) = text.split_once("\n\n").expect("missing corpus header");
                let header = header
                    .lines()
                    .map(|line| line.strip_prefix("// ").expect("invalid HQL metadata line"))
                    .collect::<Vec<_>>()
                    .join("\n");
                (header, source.to_owned())
            } else {
                let (_, rest) = text
                    .split_once("<!-- corpus\n")
                    .expect("missing HMD metadata");
                let (header, _) = rest.split_once("\n-->").expect("unclosed HMD metadata");
                (header.to_owned(), text.clone())
            };
            let mut metadata = BTreeMap::new();
            for line in header.lines() {
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
                source,
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
    let index = fs::read_to_string(root.join("README.md")).unwrap();
    let cases = load_cases();
    assert!(!cases.is_empty());
    assert_eq!(
        index.lines().filter(|line| line.starts_with("| [")).count(),
        cases.len()
    );
    let mut statuses = BTreeMap::new();
    for case in cases {
        let m = &case.metadata;
        for field in ["status", "feature", "implementation", "environment"] {
            assert!(m.contains_key(field), "{}: missing {field}", case.path);
        }
        let status = m["status"].as_str();
        assert!(matches!(
            status,
            "valid-now" | "proposed" | "invalid" | "design-question"
        ));
        *statuses.entry(status.to_owned()).or_insert(0) += 1;
        assert!(matches!(
            m["implementation"].as_str(),
            "implemented" | "pending"
        ));
        assert!(matches!(
            m["environment"].as_str(),
            "pure"
                | "prelude-v1"
                | "knowledge-v1"
                | "unbound-card-v1"
                | "modules-v1"
                | "alice-cell-v1"
                | "transclusion-v1"
        ));
        if status == "valid-now" {
            assert_eq!(m["implementation"], "implemented");
            assert!(m.contains_key("expected-type") && m.contains_key("expected"));
        }
        if status == "invalid" {
            assert!(m.contains_key("error") && m.contains_key("stage"));
        }
        if m["implementation"] == "implemented" {
            assert!(matches!(status, "valid-now" | "invalid"));
            assert_eq!(m["environment"], "pure");
            assert!(case.path.ends_with(".hql"));
        }
        let mut expected = Vec::new();
        for (key, label) in [
            ("expected-type", "type"),
            ("expected", "value"),
            ("error", "error"),
        ] {
            if let Some(value) = m.get(key) {
                expected.push(format!("{label}: {value}"));
            }
        }
        assert!(!expected.is_empty(), "{}: missing expectation", case.path);
        let expected = expected.join("; ").replace('|', "\\|");
        let row = format!(
            "| [{p}]({p}) | {status} | {feature} | {expected} | {implementation} |",
            p = case.path,
            feature = m["feature"],
            implementation = m["implementation"]
        );
        assert!(
            index.lines().any(|line| line == row),
            "missing or stale index row:\n{row}"
        );
        for key in ["alternatives", "expected-file"] {
            if let Some(paths) = m.get(key) {
                for path in paths.split(", ") {
                    assert!(
                        root.join(path).is_file(),
                        "{}: missing {key} {path}",
                        case.path
                    );
                }
            }
        }
    }
    for status in ["valid-now", "proposed", "invalid", "design-question"] {
        assert!(statuses.contains_key(status));
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
        let ty = check(&case.source).unwrap_or_else(|e| panic!("{}: {e}", case.path));
        let value = eval(&case.source).unwrap_or_else(|e| panic!("{}: {e}", case.path));
        assert_eq!(
            ty.to_string(),
            case.metadata["expected-type"],
            "{}",
            case.path
        );
        assert_eq!(
            value.to_string(),
            case.metadata["expected"],
            "{}",
            case.path
        );
    }
    assert!(count > 0);
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
        };
        assert_eq!(name, m["error"], "{}", case.path);
        assert_eq!(stage, m["stage"], "{}", case.path);
    }
    assert!(count > 0);
}
