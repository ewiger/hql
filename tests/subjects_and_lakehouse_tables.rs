//! Configured NATS subjects and Spark SQL tables (HQL-0047): typed offline
//! from the configuration, a subject read by `tap` and never by `source`,
//! and written by `sink` or `target`; a lakehouse table read by `source`
//! and written by `target` or `sink`. Nothing here reaches a server; the
//! round trips run against the containers of the `hql-demo` repository.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::fs;
use tempfile::{TempDir, tempdir};

/// The environment variables named below are never set, so no case
/// connects to anything.
const CONFIG: &str = r#"
[endpoint.events]
kind = "nats"
url_env = "HQL_TEST_UNSET_NATS_URL"
stream = "EVENTS"
subject = "events.orders"
idle_ms = 500

[endpoint.every_event]
kind = "nats"
url = "nats://localhost:4222"
stream = "EVENTS"
subject = "events.>"

[endpoint.orders]
kind = "spark"
url_env = "HQL_TEST_UNSET_LAKEHOUSE_URL"
table = "main.sales.orders"
order = ["id"]

[endpoint.orders.fields.total]
type = "DECIMAL(12,2)"

[endpoint.warehouse_orders]
kind = "spark"
url = "https://example.invalid/sql/1.0/warehouses/abc"
token_env = "HQL_TEST_UNSET_LAKEHOUSE_TOKEN"
table = "orders"

[endpoint.counts]
kind = "csv"
path = "counts.csv"
"#;

const ORDER: &str = "type Order { id : Int, customer : String, total : Decimal }\n";

fn configured() -> (TempDir, Vault) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("hql.toml"), CONFIG).unwrap();
    fs::write(dir.path().join("counts.csv"), "species,count\nwren,3\n").unwrap();
    let vault = vault::load(dir.path()).unwrap();
    (dir, vault)
}

fn refusal(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(outcome.failed(), "{source} ran");
    outcome
        .diagnostics
        .iter()
        .map(|report| report.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn checks(source: &str, vault: &Vault) {
    if let Err(error) = hql::check_in(source, vault) {
        panic!("{source}: {error:?}");
    }
}

#[test]
fn a_subject_is_tapped_as_a_stream_and_never_read_by_source() {
    let (_dir, vault) = configured();
    checks(r#"tap("events") | take(2)"#, &vault);
    checks(
        &format!("{ORDER}events : Stream<Order> = tap(\"events\")\nevents | take(1)"),
        &vault,
    );
    let error = refusal(r#"source("events")"#, &vault);
    assert!(
        error.contains("`events` is a `nats` endpoint, read by `tap` as a `Stream`"),
        "{error}"
    );
}

#[test]
fn a_lakehouse_table_is_a_seq_and_its_rows_may_be_tapped() {
    let (_dir, vault) = configured();
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | size"),
        &vault,
    );
    checks(r#"tap("orders") | take(1)"#, &vault);
}

#[test]
fn a_stream_is_sunk_into_a_subject_or_a_table_and_a_table_into_a_subject() {
    let (_dir, vault) = configured();
    // NATS to the lakehouse: each message a row, as it arrives.
    checks(
        &format!("{ORDER}events : Stream<Order> = tap(\"events\")\nevents | sink(\"orders\")"),
        &vault,
    );
    // The lakehouse to NATS: each row a message.
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | sink(\"events\")"),
        &vault,
    );
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | target(\"events\")"),
        &vault,
    );
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | target(\"orders\")"),
        &vault,
    );
    // A subject takes any document; a table takes rows of a declared record.
    checks(r#"[{a: 1}] | sink("events")"#, &vault);
    let error = refusal(r#"[{a: 1}] | sink("orders")"#, &vault);
    assert!(error.contains("rows of a declared record"), "{error}");
    let error = refusal(r#"[{a: 1}] | sink("events", Json)"#, &vault);
    assert!(error.contains("format is its kind's"), "{error}");
}

#[test]
fn a_message_is_published_to_one_subject_never_to_a_wildcard() {
    let (_dir, vault) = configured();
    checks(r#"tap("every_event") | take(1)"#, &vault);
    // Refused before any connection is made.
    let error = refusal(r#"[{a: 1}] | sink("every_event")"#, &vault);
    assert!(error.contains("`events.>` holds a wildcard"), "{error}");
}

#[test]
fn a_lakehouse_column_is_written_as_a_type_that_keeps_the_value() {
    let (_dir, vault) = configured();
    // A list has no column type of its own here.
    let error = refusal(
        "type Tagged { id : Int, tags : List<String> }\n\
         [Tagged(id = 1, tags = [\"a\"])] | sink(\"orders\")",
        &vault,
    );
    assert!(
        error.contains("`spark`'s `STRING` does not decode to List<String>"),
        "{error}"
    );
    // A Decimal in a DOUBLE column may lose digits, and says so.
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        "[endpoint.orders]\nkind = \"spark\"\nurl_env = \"U\"\ntable = \"orders\"\n\n\
         [endpoint.orders.fields.total]\ntype = \"DOUBLE\"\n",
    )
    .unwrap();
    let lossy = vault::load(dir.path()).unwrap();
    let written =
        format!("{ORDER}[Order(id = 1, customer = \"ada\", total = 1.50)] | target(\"orders\")");
    let error = refusal(&written, &lossy);
    assert!(error.contains("write `lossy = true`"), "{error}");
    // As a DECIMAL of a stated precision and scale it is written.
    checks(&written, &vault);
}

#[test]
fn an_unset_variable_fails_where_the_endpoint_is_used_naming_the_variable() {
    let (_dir, vault) = configured();
    let error = refusal(r#"tap("events") | take(1)"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_NATS_URL"), "{error}");
    let error = refusal(r#"[{a: 1}] | sink("events")"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_NATS_URL"), "{error}");
    let error = refusal(r#"source("orders") | size"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_LAKEHOUSE_URL"), "{error}");
    // The token is read before the address is called.
    let error = refusal(r#"source("warehouse_orders") | size"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_LAKEHOUSE_TOKEN"), "{error}");
}

#[test]
fn a_malformed_subject_or_lakehouse_table_refuses_the_vault() {
    for (entry, says) in [
        (
            "kind = \"nats\"\nstream = \"S\"\nsubject = \"s\"\n",
            "names its address once",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nurl_env = \"U\"\nstream = \"S\"\nsubject = \"s\"\n",
            "names its address once",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://user:pass@h:4222\"\nstream = \"S\"\nsubject = \"s\"\n",
            "url` holds credentials",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nsubject = \"s\"\n",
            "names its `stream`",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nstream = \"S\"\n",
            "names its `subject`",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nstream = \"S\"\nsubject = \"s\"\nfrom = \"middle\"\n",
            "`earliest` or `latest`",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nstream = \"S\"\nsubject = \"s\"\nidle_ms = 0\n",
            "idle_ms",
        ),
        (
            "kind = \"nats\"\nurl = \"nats://h:4222\"\nstream = \"S\"\nsubject = \"s\"\ntoken_env = \"T\"\n",
            "takes no `token_env`",
        ),
        (
            "kind = \"spark\"\ntable = \"t\"\n",
            "names its address once",
        ),
        ("kind = \"spark\"\nurl_env = \"U\"\n", "names its `table`"),
        (
            "kind = \"spark\"\nurl_env = \"U\"\ntable = \"t\"\ntoken = \"dapi123\"\n",
            "`token_env = \"VARIABLE\"`",
        ),
        (
            "kind = \"spark\"\nurl_env = \"U\"\ntable = \"t\"\ntopic = \"x\"\n",
            "takes no `topic`",
        ),
        (
            "kind = \"spark\"\nurl_env = \"U\"\ntable = \"t\"\ndsn_env = \"D\"\n",
            "takes no `dsn_env`",
        ),
    ] {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("hql.toml"),
            format!("[endpoint.db]\n{entry}"),
        )
        .unwrap();
        let error = vault::load(dir.path()).unwrap_err().to_string();
        assert!(error.contains(says), "{entry}: {error}");
    }
}

#[test]
fn a_list_field_is_written_to_a_nesting_format_as_a_list() {
    // The cell a message's boundary writes is the cell a JSON file's does.
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("hql.toml"),
        "[endpoint.rejected]\nkind = \"json_lines\"\npath = \"rejected.jsonl\"\n",
    )
    .unwrap();
    let vault = vault::load(dir.path()).unwrap();
    let outcome = hql::run(
        "type Rejected { id : Int, rules : List<String> }\n\
         [Rejected(id = 3, rules = [\"a.type\", \"b.missing\"])] | target(\"rejected\")",
        &vault,
        Mode::Strict,
    );
    assert!(!outcome.failed(), "{:?}", outcome.diagnostics);
    let written = fs::read_to_string(dir.path().join("rejected.jsonl")).unwrap();
    assert_eq!(written.trim(), r#"{"id":3,"rules":["a.type","b.missing"]}"#);
}
