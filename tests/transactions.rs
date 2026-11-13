//! Transactions and relational mutations (HQL-0051), phase 1: the
//! capabilities `Transactional`, `Insertable`, `Updatable` and `Deletable`
//! as supertypes of the table transports, `Postgres` and its constructor in
//! `io.db.postgres`, and `configured("id")`, a configured endpoint as a
//! value. Every case runs offline: the SQLite cases against a file in a
//! temporary directory, and the PostgreSQL cases against an environment
//! variable that is never set, so nothing connects.

use hql::reporting::Mode;
use hql::vault::{self, Vault};
use std::fs;
use std::path::Path;
use tempfile::{TempDir, tempdir};

const CONFIG: &str = r#"
[endpoint.ledger]
kind = "sqlite"
path = "shop.db"
table = "orders"
order = ["id"]

[endpoint.orders]
kind = "postgres"
dsn_env = "HQL_TEST_UNSET_DSN"
table = "orders"
order = ["id"]

[endpoint.events]
kind = "kafka"
brokers_env = "HQL_TEST_UNSET_BROKERS"
topic = "events"
idle_ms = 500

[endpoint.counts]
kind = "csv"
path = "counts.csv"

[endpoint.notes]
kind = "vault"
root = "notes"
"#;

const ORDER: &str =
    "type Order { id : Int, customer : String, total : Decimal, paid : Bool }\n";

fn configured() -> (TempDir, Vault) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("hql.toml"), CONFIG).unwrap();
    fs::write(dir.path().join("counts.csv"), "species,count\nwren,3\n").unwrap();
    fs::create_dir(dir.path().join("notes")).unwrap();
    let vault = vault::load(dir.path()).unwrap();
    (dir, vault)
}

/// A program's value as compact JSON.
fn value(source: &str, vault: &Vault) -> String {
    let outcome = hql::run(source, vault, Mode::Strict);
    assert!(!outcome.failed(), "{source}: {:?}", outcome.diagnostics);
    outcome
        .value
        .map(|value| value.to_string())
        .unwrap_or_default()
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

fn checked(source: &str, vault: &Vault) -> String {
    match hql::check_in(source, vault) {
        Ok(ty) => ty.to_string(),
        Err(error) => panic!("{source}: {error:?}"),
    }
}

/// The ledger's table, written through the configured endpoint.
fn seeded(dir: &Path, vault: &Vault) {
    value(
        &format!(
            "{ORDER}\
             [Order(id = 2, customer = \"grace\", total = 42.00, paid = false), \
              Order(id = 1, customer = \"ada\", total = 120.50, paid = true)]\n\
             | target(\"ledger\")"
        ),
        vault,
    );
    assert!(dir.join("shop.db").exists());
}

#[test]
fn the_table_transports_declare_the_four_capabilities_and_a_file_declares_none() {
    let (_dir, vault) = configured();
    for (module, constructor) in [
        ("io.db.sqlite", "sqlite(\"x.db\", table = \"t\")"),
        (
            "io.db.postgres",
            "postgres(dsn_env = \"D\", table = \"t\")",
        ),
    ] {
        for capability in ["Transactional", "Insertable", "Updatable", "Deletable"] {
            let program = format!("import {module}\nt : {capability} = {constructor}\nt");
            assert_eq!(checked(&program, &vault), capability, "{program}");
        }
    }
    for capability in ["Transactional", "Insertable", "Updatable", "Deletable"] {
        let error = refusal(
            &format!("t : {capability} = file(\"a.json\")\nt"),
            &vault,
        );
        assert!(error.contains(capability), "{error}");
    }
    // A capability is independent of the others: each is an abstract type
    // below `Transport`, and none narrows another.
    for (held, other) in [("Transactional", "Updatable"), ("Insertable", "Deletable")] {
        let error = refusal(
            &format!(
                "import io.db.postgres\n\
                 t : {held} = postgres(dsn_env = \"D\", table = \"t\")\n\
                 u : {other} = t\nu"
            ),
            &vault,
        );
        assert!(error.contains(other), "{error}");
    }
}

#[test]
fn the_isolation_levels_are_types_below_isolation_level() {
    let (_dir, vault) = configured();
    // A declaration may restate a built-in type and not contradict it.
    for level in ["ReadCommitted", "RepeatableRead", "Serializable"] {
        assert_eq!(
            checked(&format!("type {level} : IsolationLevel"), &vault),
            "Unit",
            "{level}"
        );
        let error = refusal(&format!("type {level} : LogLevel\n1"), &vault);
        assert!(error.contains("contradicts"), "{error}");
    }
}

#[test]
fn postgres_is_constructed_by_its_module_and_is_a_network_row_transport() {
    let (_dir, vault) = configured();
    let error = refusal(
        r#"source(postgres(dsn_env = "D", table = "t"))"#,
        &vault,
    );
    assert!(error.contains("import io.db.postgres"), "{error}");
    assert_eq!(
        checked(
            "import io.db.postgres\npostgres(dsn_env = \"D\", table = \"t\")",
            &vault
        ),
        "Postgres"
    );
    for supertype in ["Network", "RowTransport", "Replayable", "Writable"] {
        assert_eq!(
            checked(
                &format!(
                    "import io.db.postgres\n\
                     postgres(dsn_env = \"D\", table = \"t\") | is({supertype})"
                ),
                &vault,
            ),
            "Bool",
            "{supertype}"
        );
    }
    assert_eq!(
        checked(
            "import io.db.postgres\nsource(postgres(dsn_env = \"D\", table = \"t\", order = [\"id\"]))",
            &vault
        ),
        "Seq<Data>"
    );
    assert_eq!(
        checked(
            &format!(
                "import io.db.postgres\n{ORDER}\
                 orders : Stream<Order> = tap(postgres(dsn_env = \"D\", table = \"t\"))\norders"
            ),
            &vault
        ),
        "Stream<Order>"
    );
    // A row transport takes no format, and is written whole by `target`.
    let error = refusal(
        "import io.db.postgres\nsource(postgres(dsn_env = \"D\", table = \"t\"), Json)",
        &vault,
    );
    assert!(error.contains("rows of named columns, not bytes"), "{error}");
    let error = refusal(
        &format!(
            "import io.db.postgres\n{ORDER}\
             [Order(id = 1, customer = \"a\", total = 1.0, paid = true)] \
             | sink(postgres(dsn_env = \"D\", table = \"t\"))"
        ),
        &vault,
    );
    assert!(error.contains("written whole by `target`"), "{error}");
    // The connection string is read where the endpoint is used, from the
    // variable the constructor names; an unset one fails naming it.
    let error = refusal(
        "import io.db.postgres\n\
         source(postgres(dsn_env = \"HQL_TEST_UNSET_DSN\", table = \"t\")) | size",
        &vault,
    );
    assert!(error.contains("HQL_TEST_UNSET_DSN"), "{error}");
}

#[test]
fn a_configured_id_is_typed_offline_as_its_kinds_transport() {
    let (_dir, vault) = configured();
    assert_eq!(checked(r#"configured("orders")"#, &vault), "Postgres");
    assert_eq!(checked(r#"configured("ledger")"#, &vault), "Sqlite");
    assert_eq!(checked(r#"configured("counts")"#, &vault), "File");
    assert_eq!(
        checked(r#"configured("orders") | is(Transactional)"#, &vault),
        "Bool"
    );
    assert_eq!(
        checked(r#"t : Set<Transactional> = {configured("orders")}"#, &vault),
        "Unit"
    );
    let error = refusal(r#"configured("ordres")"#, &vault);
    assert!(error.contains("did you mean `orders`"), "{error}");
    let error = refusal(r#"configured("notes")"#, &vault);
    assert!(error.contains("is a vault"), "{error}");
    let error = refusal(r#"configured("events")"#, &vault);
    assert!(error.contains("has no type until its module"), "{error}");
    let error = refusal(r#"id = "orders"
configured(id)"#, &vault);
    assert!(error.contains("literal id"), "{error}");
    let error = refusal(r#"configured("postgres://db/orders")"#, &vault);
    assert!(error.contains("not an endpoint id"), "{error}");
}

#[test]
fn a_configured_value_is_read_and_written_as_its_id_is() {
    let (dir, vault) = configured();
    seeded(dir.path(), &vault);
    assert_eq!(
        checked(r#"source(configured("ledger"))"#, &vault),
        checked(r#"source("ledger")"#, &vault)
    );
    assert_eq!(
        checked(r#"tap(configured("ledger"))"#, &vault),
        checked(r#"tap("ledger")"#, &vault)
    );
    let by_value = value(
        &format!("{ORDER}orders : Seq<Order> = source(configured(\"ledger\"))\norders | map(o => [o.id, o.total])"),
        &vault,
    );
    let by_id = value(
        &format!("{ORDER}orders : Seq<Order> = source(\"ledger\")\norders | map(o => [o.id, o.total])"),
        &vault,
    );
    assert_eq!(by_value, by_id);
    assert_eq!(by_value, "[[1, 120.50], [2, 42.00]]");
    assert_eq!(
        value(r#"source(configured("ledger")) | map(r => r.customer)"#, &vault),
        r#"["ada", "grace"]"#
    );
    assert_eq!(
        value(r#"tap(configured("ledger")) | map(r => r.id) | take(1)"#, &vault),
        "[1]"
    );
    // Bound once, the value is passed as any endpoint is.
    assert_eq!(
        value(
            "ledger = configured(\"ledger\")\nsource(ledger) | size",
            &vault
        ),
        "2"
    );
    // A write through the value is the configured write, mode and boundary
    // included: `replace` leaves only the rows written.
    value(
        &format!(
            "{ORDER}\
             [Order(id = 9, customer = \"linus\", total = 7.99, paid = true)]\n\
             | target(configured(\"ledger\"))"
        ),
        &vault,
    );
    assert_eq!(
        value(r#"source("ledger") | map(r => r.id)"#, &vault),
        "[9]"
    );
    // The checks of a configured write apply: a format is refused, and so
    // is a vault.
    let error = refusal(
        &format!(
            "{ORDER}[Order(id = 1, customer = \"a\", total = 1.0, paid = true)] \
             | target(configured(\"ledger\"), Json)"
        ),
        &vault,
    );
    assert!(error.contains("format is its kind's"), "{error}");
    // A configured PostgreSQL endpoint reads through its id's path, and an
    // unset secret fails naming the variable.
    let error = refusal(r#"source(configured("orders")) | size"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_DSN"), "{error}");
    // A kafka endpoint has no value yet, by its id or by `configured`.
    let error = refusal(r#"source("events")"#, &vault);
    assert!(error.contains("read by `tap`"), "{error}");
}
