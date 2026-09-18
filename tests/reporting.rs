//! The reporting layer: severities, the queue, and where a mode comes from.

use hql::reporting::{Mode, Settings, Severity};
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
