//! The reporting layer: severities, the queue, and where a mode comes from.

use hql::reporting::{Level, Mode, Settings, Severity};
use hql::vault::Vault;
use hql::{check_reported, run};
use std::fs;
use tempfile::tempdir;

#[test]
fn strict_stops_at_the_first_error_and_collect_reports_them_all() {
    let source = "a = nosuchthing\nb = alsomissing\nc = stillmissing";

    let strict = check_reported(source, &Vault::empty(), Mode::Strict);
    assert_eq!(strict.reports.len(), 1);

    let collected = check_reported(source, &Vault::empty(), Mode::Collect);
    assert_eq!(collected.reports.len(), 3);
    assert!(
        collected
            .reports
            .iter()
            .all(|report| report.severity == Severity::Error)
    );
}

#[test]
fn a_statement_that_fails_does_not_cascade_into_the_ones_after_it() {
    // `a` did not check, but it is still bound, so the second statement is
    // reported on its own fault rather than on a missing name.
    let outcome = check_reported("a = nosuchthing\nb = a + 1", &Vault::empty(), Mode::Collect);
    assert_eq!(outcome.reports.len(), 2);
    let messages: Vec<String> = outcome
        .reports
        .iter()
        .map(|report| report.message.clone())
        .collect();
    assert!(messages[0].contains("nosuchthing"), "{messages:?}");
    assert!(messages[1].contains("addition"), "{messages:?}");
}

#[test]
fn a_syntax_failure_stops_even_in_collect_mode() {
    // Nothing after an unparseable source can be examined, so collecting has
    // nothing to collect.
    let outcome = check_reported("1 +", &Vault::empty(), Mode::Collect);
    assert_eq!(outcome.reports.len(), 1);
    assert_eq!(outcome.reports.worst(), Some(Severity::Failure));
}

#[test]
fn a_value_and_a_queue_come_back_together() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    let outcome = run("[[nowhere]]\n1 + 1", &vault, Mode::Strict);
    // The warning did not stop the program, and the result is still there.
    assert!(outcome.value.is_some());
    assert!(!outcome.failed());
    assert_eq!(outcome.reports.count(Severity::Warning), 1);
}

#[test]
fn the_queue_is_walked_oldest_first() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    let outcome = run(
        "[[first-missing]]\n[[second-missing]]\n1",
        &vault,
        Mode::Strict,
    );
    let mut reports = outcome.reports;
    assert!(
        reports
            .next_report()
            .unwrap()
            .message
            .contains("first-missing")
    );
    assert!(
        reports
            .next_report()
            .unwrap()
            .message
            .contains("second-missing")
    );
    assert!(reports.next_report().is_none());
}

#[test]
fn a_vault_states_its_own_mode_and_the_query_still_outranks_it() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join(hql::reporting::CONFIG_FILE),
        "[report]\nmode = \"collect\"\n",
    )
    .unwrap();

    let from_vault = Settings::resolve(None, Some(directory.path()));
    assert_eq!(from_vault.mode, Mode::Collect);
    assert!(from_vault.origin.contains(hql::reporting::CONFIG_FILE));

    let from_query = Settings::resolve(Some(Mode::Strict), Some(directory.path()));
    assert_eq!(from_query.mode, Mode::Strict);
    assert_eq!(from_query.origin, "this query");
}

#[test]
fn a_vault_without_a_configuration_file_falls_through() {
    let directory = tempdir().unwrap();
    let settings = Settings::resolve(None, Some(directory.path()));
    assert_eq!(settings.mode, Mode::Strict);
}

fn fixture() -> Vault {
    hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap()
}

#[test]
fn debug_is_the_identity_and_logs_one_entry() {
    let vault = fixture();
    let with = run(
        "cards | debug(label = \"all\") | count",
        &vault,
        Mode::Strict,
    );
    let without = run("cards | count", &vault, Mode::Strict);
    assert_eq!(with.value, without.value);
    assert_eq!(with.inferred, without.inferred);
    assert_eq!(with.reports.len(), 1);
    let entry = with.reports.iter().next().unwrap();
    assert_eq!(entry.severity, Severity::Debug);
    assert_eq!(entry.code.as_deref(), Some("debug"));
    assert_eq!(entry.label.as_deref(), Some("all"));
    assert_eq!(entry.message, "Set<Card>, 6 elements");
    assert_eq!(entry.ty.as_deref(), Some("Set<Card>"));
    // The span is the step, not the pipeline that feeds it.
    assert_eq!(entry.span, 8..28);
    // Below the warning level, so the run neither fails nor errs.
    assert!(!with.failed());
}

#[test]
fn debug_inside_a_lambda_logs_once_per_element_with_its_trace() {
    let outcome = run(
        "[3, 1, 2] | map(n => debug(n + 1, level = \"info\"))",
        &Vault::empty(),
        Mode::Strict,
    );
    assert_eq!(outcome.value.unwrap().to_string(), "[4, 2, 3]");
    let entries: Vec<_> = outcome.reports.iter().collect();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|entry| entry.severity == Severity::Info));
    let messages: Vec<&str> = entries.iter().map(|entry| entry.message.as_str()).collect();
    assert_eq!(messages, ["4", "2", "3"]);
    let trace = &entries[0].trace;
    assert_eq!(trace[0].step, "map");
    assert_eq!(trace[0].element.as_deref(), Some("3"));
    assert_eq!(trace[1].step, "debug");
    let json = outcome.reports.to_json();
    assert_eq!(json[0]["found"], 4);
    assert_eq!(json[0]["trace"][0]["element"], "3");
}

#[test]
fn debug_keeps_absence_and_refuses_a_level_that_reports_a_problem() {
    let outcome = run("[[nowhere]] | debug", &Vault::empty(), Mode::Strict);
    assert_eq!(outcome.inferred.unwrap().to_string(), "Option<Card>");
    assert_eq!(outcome.value.unwrap().to_string(), "none");
    for source in ["1 | debug(level = \"warning\")", "1 | debug(label = 2)"] {
        let refused = check_reported(source, &Vault::empty(), Mode::Strict);
        assert_eq!(refused.reports.worst(), Some(Severity::Error), "{source}");
    }
}

#[test]
fn entries_captured_before_a_failure_survive_it() {
    let outcome = run("x = 1 | debug\n1 / 0", &Vault::empty(), Mode::Strict);
    assert!(outcome.failed());
    assert_eq!(outcome.reports.count(Severity::Debug), 1);
    assert_eq!(outcome.reports.count(Severity::Failure), 1);
}

#[test]
fn the_shown_level_filters_without_discarding() {
    let outcome = run("[[nowhere]] | debug", &Vault::empty(), Mode::Strict);
    assert_eq!(outcome.reports.len(), 2);
    assert_eq!(outcome.reports.shown(Severity::Warning).len(), 1);
    assert_eq!(outcome.reports.shown(Severity::Debug).len(), 2);
    assert_eq!(Severity::parse_level("Info"), Some(Severity::Info));
    assert_eq!(Severity::parse_level("failure"), None);
    assert!(Severity::Debug < Severity::Info && Severity::Info < Severity::Warning);
}

#[test]
fn the_shown_level_comes_from_the_query_then_the_vault() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("hql.toml"),
        "[report]\nlevel = \"info\"\n",
    )
    .unwrap();
    let from_vault = Level::resolve(None, Some(directory.path()));
    assert_eq!(from_vault.severity, Severity::Info);
    assert!(from_vault.origin.contains("hql.toml"));
    let from_query = Level::resolve(Some(Severity::Debug), Some(directory.path()));
    assert_eq!(from_query.severity, Severity::Debug);
    assert_eq!(from_query.origin, "this query");
}
