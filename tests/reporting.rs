//! The error model (HQL-0045): the four axes of a diagnostic, the run's
//! diagnostic channel, continuation by phase, and where a mode comes from.

use hql::diagnostics::{self, Diagnostic, Disposition, Kind, Phase, Severity};
use hql::reporting::{Level, Mode, Settings};
use hql::vault::Vault;
use hql::{check_reported, run};
use std::fs;
use tempfile::tempdir;

fn axes(diagnostic: &Diagnostic) -> (Phase, Kind, Severity, Disposition) {
    (
        diagnostic.phase,
        diagnostic.kind,
        diagnostic.severity,
        diagnostic.disposition,
    )
}

#[test]
fn each_phase_states_its_own_axes() {
    let parse = run("1 +", &Vault::empty(), Mode::Strict);
    assert_eq!(
        axes(&parse.diagnostics[0]),
        (
            Phase::Parse,
            Kind::UnexpectedToken,
            Severity::Failure,
            Disposition::Fatal
        )
    );
    let check = run("\"a\" + 1", &Vault::empty(), Mode::Strict);
    assert_eq!(
        axes(&check.diagnostics[0]),
        (
            Phase::Check,
            Kind::TypeMismatch,
            Severity::Error,
            Disposition::Fatal
        )
    );
    let execute = run("[1, 0] | map(n => 1 / n)", &Vault::empty(), Mode::Strict);
    assert_eq!(
        axes(&execute.diagnostics[0]),
        (
            Phase::Execute,
            Kind::DivisionByZero,
            Severity::Failure,
            Disposition::Fatal
        )
    );
    let json = execute.diagnostics[0].to_json();
    assert_eq!(json["phase"], "execute");
    assert_eq!(json["kind"], "division-by-zero");
    assert_eq!(json["element"]["position"], 2);
    assert_eq!(json["element"]["value"], 0);
}

#[test]
fn severity_is_not_derived_from_the_phase() {
    // A contract read and a division by zero are both execute-phase
    // failures, and their producers rate them differently.
    let contract = run(
        "type User { age : Int }\nData(\"{\\\"age\\\": \\\"x\\\"}\", Json) | User",
        &Vault::empty(),
        Mode::Strict,
    );
    let read = &contract.diagnostics[0];
    assert_eq!(
        (read.phase, read.kind, read.severity),
        (Phase::Execute, Kind::ContractViolation, Severity::Error)
    );
    assert_eq!(read.context.expected.as_deref(), Some("User"));
    assert_eq!(read.context.violations.len(), 1);
    let division = run("1 / 0", &Vault::empty(), Mode::Strict);
    assert_eq!(division.diagnostics[0].severity, Severity::Failure);
}

#[test]
fn one_channel_holds_every_phase_in_order() {
    // A check warning, then an execute failure: written in that order, and
    // the warning is not lost when the run fails.
    let outcome = run(
        "fn totalOf(xs : List<Int>) : Int = xs | sum\ntotalOf([1]) / 0",
        &Vault::empty(),
        Mode::Strict,
    );
    let order: Vec<(Phase, Kind)> = outcome
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.phase, diagnostic.kind))
        .collect();
    assert_eq!(
        order,
        [
            (Phase::Check, Kind::NamingConvention),
            (Phase::Execute, Kind::DivisionByZero)
        ]
    );
    assert_eq!(outcome.diagnostics[0].context.help, ["write it `total_of`"]);
    assert!(outcome.failed());
}

#[test]
fn a_warning_leaves_the_value_and_the_status() {
    let vault = hql::vault::load(std::path::Path::new("tests/fixtures/vault")).unwrap();
    let outcome = run("@[[nowhere]]\n1 + 1", &vault, Mode::Strict);
    assert!(outcome.value.is_some());
    assert!(!outcome.failed());
    let warning = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.phase != Phase::Load)
        .unwrap();
    assert_eq!(
        axes(warning),
        (
            Phase::Execute,
            Kind::UnresolvedLink,
            Severity::Warning,
            Disposition::Warning
        )
    );
}

#[test]
fn the_loader_writes_skipped_document_and_keeps_the_rest() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("hql.toml"), "").unwrap();
    fs::write(
        directory.path().join("broken.md"),
        "---\ntitle: [unclosed\n---\n# Broken\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("fine.md"),
        "---\ntitle: Fine\n---\n# Fine\n",
    )
    .unwrap();
    let vault = hql::vault::load(directory.path()).unwrap();
    let outcome = run("cards | size", &vault, Mode::Strict);
    assert_eq!(
        outcome.value.as_ref().map(|value| value.to_string()),
        Some("1".into())
    );
    assert!(!outcome.failed());
    let skipped = &outcome.diagnostics[0];
    assert_eq!(
        (skipped.phase, skipped.kind, skipped.disposition),
        (Phase::Load, Kind::SkippedDocument, Disposition::Recoverable)
    );
    assert!(
        skipped
            .origin
            .as_deref()
            .is_some_and(|origin| origin.ends_with("broken.md"))
    );
}

#[test]
fn collect_steps_past_an_element_and_stops_at_an_overflow() {
    let source = "[6, 0, 3] | map(n => 6 / n)";
    let strict = run(source, &Vault::empty(), Mode::Strict);
    assert!(strict.value.is_none());
    assert_eq!(strict.diagnostics[0].disposition, Disposition::Fatal);

    let collected = run(source, &Vault::empty(), Mode::Collect);
    assert_eq!(
        collected.value.as_ref().map(|value| value.to_string()),
        Some("[1, 2]".into())
    );
    assert_eq!(collected.diagnostics.len(), 1);
    assert_eq!(
        collected.diagnostics[0].disposition,
        Disposition::Recoverable
    );
    assert!(collected.failed(), "a failure was written");

    let overflow = run(
        "[1, 9223372036854775807] | map(n => n + 1)",
        &Vault::empty(),
        Mode::Collect,
    );
    assert!(overflow.value.is_none());
    assert_eq!(overflow.diagnostics[0].kind, Kind::NumericOverflow);
    assert_eq!(overflow.diagnostics[0].disposition, Disposition::Fatal);
}

#[test]
fn a_data_failure_writes_no_diagnostic() {
    let outcome = run(
        "type User { age : Int }\n[{age: 1}, {age: \"x\"}] | map(u => u | validate(User)) | errs() | size",
        &Vault::empty(),
        Mode::Strict,
    );
    assert_eq!(
        outcome.value.as_ref().map(|value| value.to_string()),
        Some("1".into())
    );
    assert!(outcome.diagnostics.is_empty());
}

#[test]
fn strict_stops_at_the_first_error_and_collect_checks_them_all() {
    let source = "a = nosuchthing\nb = alsomissing\nc = stillmissing";

    let strict = check_reported(source, &Vault::empty(), Mode::Strict);
    assert_eq!(strict.diagnostics.len(), 1);
    assert_eq!(strict.diagnostics[0].disposition, Disposition::Fatal);

    let collected = check_reported(source, &Vault::empty(), Mode::Collect);
    assert_eq!(collected.diagnostics.len(), 3);
    assert!(collected.diagnostics.iter().all(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.disposition == Disposition::Recoverable
    }));
}

#[test]
fn a_statement_that_fails_does_not_cascade_into_the_ones_after_it() {
    // `a` did not check, but it is still bound, so the second statement is
    // reported on its own fault rather than on a missing name.
    let outcome = check_reported("a = nosuchthing\nb = a + 1", &Vault::empty(), Mode::Collect);
    assert_eq!(outcome.diagnostics.len(), 2);
    assert!(outcome.diagnostics[0].message.contains("nosuchthing"));
    assert!(outcome.diagnostics[1].message.contains("addition"));
}

#[test]
fn a_syntax_failure_stops_even_in_collect_mode() {
    let outcome = check_reported("1 +", &Vault::empty(), Mode::Collect);
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(
        diagnostics::worst(&outcome.diagnostics),
        Some(Severity::Failure)
    );
    assert_eq!(outcome.diagnostics[0].disposition, Disposition::Fatal);
}

#[test]
fn a_nearest_name_is_help_and_not_the_message() {
    let outcome = run("[1, 2] | mapp(n => n)", &Vault::empty(), Mode::Strict);
    let unknown = &outcome.diagnostics[0];
    assert_eq!(unknown.kind, Kind::UnknownStep);
    assert_eq!(unknown.message, "`mapp` is not a pipeline step");
    assert_eq!(unknown.context.help, ["did you mean `map`?"]);
}

#[test]
fn a_second_place_is_a_label() {
    let source = "trait T { fn f(self) : Int }\ntype A { x : Int }\n\
                  impl T for A { fn f(self) : Int = 1 }\nimpl T for A { fn f(self) : Int = 2 }\n1";
    let outcome = run(source, &Vault::empty(), Mode::Strict);
    let duplicate = &outcome.diagnostics[0];
    assert_eq!(duplicate.kind, Kind::DuplicateImplementation);
    assert!(!duplicate.message.contains("byte"), "{}", duplicate.message);
    assert_eq!(duplicate.context.labels.len(), 1);
    assert!(duplicate.context.labels[0].span.start < duplicate.span.start);
}

#[test]
fn a_tap_observes_the_channel_as_it_is_written() {
    use std::sync::{Arc, Mutex};
    assert!(hql::io::open_diagnostics());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tapped = Arc::clone(&seen);
    hql::io::tap_diagnostics(move |diagnostic| {
        tapped.lock().unwrap().push(diagnostic.kind);
    });
    let outcome = run("x = @[[nowhere]]\n1 / 0", &Vault::empty(), Mode::Strict);
    // The run did not own the channel, so it left it open; the host closes
    // it, and its contents are what the run returned.
    let closed = hql::io::close_diagnostics();
    assert_eq!(closed, outcome.diagnostics);
    assert_eq!(
        *seen.lock().unwrap(),
        [Kind::UnresolvedLink, Kind::DivisionByZero]
    );
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

#[test]
fn the_shown_level_parses_by_name() {
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
