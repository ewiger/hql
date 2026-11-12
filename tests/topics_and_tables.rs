//! Configured Kafka topics and PostgreSQL tables (HQL-0046): typed offline
//! from the configuration, a topic read by `tap` and never by `source`, and
//! written by `sink` or `target`. Nothing here reaches a broker or a
//! database; the round trips run against the containers of the `hql-demo`
//! repository.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::fs;
use tempfile::{TempDir, tempdir};

/// The environment variables named below are never set, so no case
/// connects to anything.
const CONFIG: &str = r#"
[sources.events]
kind = "kafka"
brokers_env = "HQL_TEST_UNSET_BROKERS"
topic = "events"
idle_ms = 500

[sources.orders]
kind = "postgres"
dsn_env = "HQL_TEST_UNSET_DSN"
table = "orders"
order = ["id"]

[sources.counts]
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
fn a_topic_is_tapped_as_a_stream_and_never_read_by_source() {
    let (_dir, vault) = configured();
    checks(r#"tap("events") | take(2)"#, &vault);
    checks(
        &format!("{ORDER}events : Stream<Order> = tap(\"events\")\nevents | take(1)"),
        &vault,
    );
    let error = refusal(r#"source("events")"#, &vault);
    assert!(
        error.contains("`events` is a `kafka` source, read by `tap` as a `Stream`"),
        "{error}"
    );
}

#[test]
fn a_table_is_a_seq_and_its_rows_may_be_tapped() {
    let (_dir, vault) = configured();
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | size"),
        &vault,
    );
    checks(r#"tap("orders") | take(1)"#, &vault);
}

#[test]
fn a_stream_is_sunk_into_a_topic_or_a_table_and_a_table_into_a_topic() {
    let (_dir, vault) = configured();
    // Kafka to Postgres: each message a row, as it arrives.
    checks(
        &format!("{ORDER}events : Stream<Order> = tap(\"events\")\nevents | sink(\"orders\")"),
        &vault,
    );
    // Postgres to Kafka: each row a message.
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | sink(\"events\")"),
        &vault,
    );
    checks(
        &format!("{ORDER}orders : Seq<Order> = source(\"orders\")\norders | target(\"events\")"),
        &vault,
    );
    // A topic takes any document; a table takes rows of a declared record.
    checks(r#"[{a: 1}] | sink("events")"#, &vault);
    let error = refusal(r#"[{a: 1}] | sink("orders")"#, &vault);
    assert!(error.contains("rows of a declared record"), "{error}");
}

#[test]
fn sink_writes_only_a_topic_or_a_table_and_takes_no_format() {
    let (_dir, vault) = configured();
    let error = refusal(r#"[{a: 1}] | sink("counts")"#, &vault);
    assert!(error.contains("which `sink` does not write"), "{error}");
    let error = refusal(r#"[{a: 1}] | sink("events", Json)"#, &vault);
    assert!(error.contains("format is its kind's"), "{error}");
    let error = refusal(r#"[{a: 1}] | sink("evnts")"#, &vault);
    assert!(error.contains("did you mean `events`"), "{error}");
}

#[test]
fn an_unset_secret_fails_where_the_source_is_used_naming_the_variable() {
    let (_dir, vault) = configured();
    let error = refusal(r#"source("orders") | size"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_DSN"), "{error}");
    let error = refusal(r#"tap("events") | take(1)"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_BROKERS"), "{error}");
}

#[test]
fn a_malformed_topic_or_table_refuses_the_vault() {
    for (entry, says) in [
        ("kind = \"postgres\"\ntable = \"t\"\n", "dsn_env"),
        (
            "kind = \"postgres\"\ntable = \"t\"\ndsn = \"postgres://u:p@h/db\"\n",
            "`dsn_env = \"VARIABLE\"`",
        ),
        (
            "kind = \"postgres\"\ndsn_env = \"D\"\n",
            "names its `table`",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\n",
            "names its brokers once",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\nbrokers = \"a:9092\"\nbrokers_env = \"B\"\n",
            "names its brokers once",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\nbrokers = \"a:9092\"\nfrom = \"middle\"\n",
            "`earliest` or `latest`",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\nbrokers = \"a:9092\"\nidle_ms = 0\n",
            "idle_ms",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\nbrokers = \"a:9092\"\ntable = \"x\"\n",
            "takes no `table`",
        ),
        (
            "kind = \"kafka\"\ntopic = \"t\"\nbrokers = \"a:9092\"\ntoken_env = \"T\"\n",
            "takes no `token_env`",
        ),
    ] {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("hql.toml"),
            format!("[sources.db]\n{entry}"),
        )
        .unwrap();
        let error = vault::load(dir.path()).unwrap_err().to_string();
        assert!(error.contains(says), "{entry}: {error}");
    }
}
