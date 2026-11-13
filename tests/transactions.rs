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

[endpoint.accounts]
kind = "sqlite"
path = "shop.db"
table = "accounts"
order = ["id"]
element = "Account"

[endpoint.orders]
kind = "postgres"
dsn_env = "HQL_TEST_UNSET_DSN"
table = "orders"
order = ["id"]

[endpoint.transfers]
kind = "sqlite"
path = "shop.db"
table = "transfers"
order = ["id"]
element = "Transfer"

[endpoint.elsewhere]
kind = "sqlite"
path = "other.db"
table = "accounts"
order = ["id"]
element = "Account"

[endpoint.lake]
kind = "spark"
url_env = "HQL_TEST_UNSET_LAKE"
table = "default.orders"
order = ["id"]
element = "Account"

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

// Phase 2: the mutation statements over SQLite, run, and over PostgreSQL,
// checked offline.

const ACCOUNT: &str =
    "type Account { id : Int, owner : String, balance : Int, status? : String }\n";

/// The accounts table, written through the configured endpoint.
fn ledger(vault: &Vault) {
    value(
        &format!(
            "{ACCOUNT}\
             [Account(id = 1, owner = \"ada\", balance = 250, status = \"open\"), \
              Account(id = 2, owner = \"grace\", balance = 40), \
              Account(id = 3, owner = \"linus\", balance = 9, status = \"closed\")]\n\
             | target(\"accounts\")"
        ),
        vault,
    );
}

fn balances(vault: &Vault) -> String {
    value(
        &format!("{ACCOUNT}rows : Seq<Account> = source(\"accounts\")\nrows | map(a => [a.id, a.balance])"),
        vault,
    )
}

#[test]
fn update_delete_and_insert_change_rows_in_place_and_count_them() {
    let (_dir, vault) = configured();
    ledger(&vault);
    // Each statement is one of the system's, run where it is written, and
    // its value is the number of rows it changed.
    let counts = value(
        &format!(
            "{ACCOUNT}\
             accounts = configured(\"accounts\")\n\
             moved = update accounts | filter(a => a.id == 1) | set(a => {{balance: a.balance - 100}})\n\
             gone = delete accounts | filter(a => a.status == \"closed\")\n\
             added = insert accounts Account(id = 4, owner = \"mary\", balance = 1)\n\
             [moved, gone, added]"
        ),
        &vault,
    );
    assert_eq!(counts, "[1, 1, 1]");
    assert_eq!(balances(&vault), "[[1, 150], [2, 40], [4, 1]]");
    // A tree is read as the endpoint's element, and several rows are
    // inserted by one statement; a statement is also written on its own.
    assert_eq!(
        value(
            &format!("{ACCOUNT}insert configured(\"accounts\") [{{id: 5, owner: \"eve\", balance: 2}}, {{id: 6, owner: \"bob\", balance: 3}}]"),
            &vault,
        ),
        "2"
    );
    assert_eq!(
        value(
            &format!("{ACCOUNT}update configured(\"accounts\") | set(a => {{balance: a.balance + 1}})"),
            &vault
        ),
        "5"
    );
    assert_eq!(balances(&vault), "[[1, 151], [2, 41], [4, 2], [5, 3], [6, 4]]");
    assert_eq!(
        value(&format!("{ACCOUNT}delete configured(\"accounts\")"), &vault),
        "5"
    );
    assert_eq!(balances(&vault), "[]");
}

#[test]
fn a_constructed_table_is_typed_by_of_type_and_a_clause_translates_what_it_may() {
    let (dir, vault) = configured();
    ledger(&vault);
    let path = dir.path().join("shop.db").display().to_string();
    // `contains` of a bound collection, `is_none` of a field, a captured
    // binding and arithmetic are translated; the rows of a constructed table
    // are `Data` until `of_type` names them.
    let counts = value(
        &format!(
            "import io.db.sqlite\n{ACCOUNT}\
             t = sqlite({path:?}, table = \"accounts\")\n\
             ids = [2, 3]\n\
             bonus = 10\n\
             a = update t | of_type(Account) | filter(a => contains(ids, a.id) and a.status.is_none()) \
             | set(a => {{status: \"open\", balance: a.balance * 2 + bonus}})\n\
             b = delete t | of_type(Account) | filter(a => a.owner == \"nobody\" or a.balance < 0)\n\
             c = update t | of_type(Account) | filter(a => a.id == 3) | set(a => {{status: \"archived\"}})\n\
             [a, b, c]"
        ),
        &vault,
    );
    assert_eq!(counts, "[1, 0, 1]");
    assert_eq!(
        value(
            &format!("{ACCOUNT}rows : Seq<Account> = source(\"accounts\")\nrows | map(a => [a.id, a.balance, a.status | unwrap_or(\"\")])"),
            &vault
        ),
        r#"[[1, 250, "open"], [2, 90, "open"], [3, 9, "archived"]]"#
    );
    // A tree's field is an `Option<Data>`, so arithmetic over it needs the
    // row's type, as it does anywhere.
    let error = refusal(
        &format!(
            "import io.db.sqlite\n\
             update sqlite({path:?}, table = \"accounts\") | set(a => {{balance: a.balance - 1}})"
        ),
        &vault,
    );
    assert!(error.contains("Option<Data>"), "{error}");
    // The pipeline is closed, and a clause holds only what translates.
    for (program, says) in [
        (
            format!("{ACCOUNT}update configured(\"accounts\") | filter(a => size(a.owner) > 3) | set(a => {{balance: 0}})"),
            "not translated",
        ),
        (
            format!("{ACCOUNT}delete configured(\"accounts\") | filter(a => a.owner.upper() == \"ADA\")"),
            "not translated",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | set(a => {{owner: 1}})"),
            "`owner` of Account is String",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | set(a => {{colour: \"red\"}})"),
            "declares no field `colour`",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | filter(a => a.id) | set(a => {{balance: 0}})"),
            "gives a Bool",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | set(a => a.balance)"),
            "as a tree",
        ),
        (
            format!("{ACCOUNT}delete configured(\"accounts\") | map(a => a.id)"),
            "is not a clause",
        ),
        (
            format!("{ACCOUNT}delete configured(\"accounts\") | set(a => {{balance: 0}})"),
            "`set` assigns, which `delete` does not",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | filter(a => a.id == 1)"),
            "an `update` ends in `set`",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | filter(a => a.id == 1) | of_type(Account) | set(a => {{balance: 0}})"),
            "`of_type` names the rows' type first",
        ),
        (
            format!("{ACCOUNT}update configured(\"accounts\") | set(a => {{balance: 0}}) | filter(a => a.id == 1)"),
            "`set` is the last clause",
        ),
        (
            format!("{ACCOUNT}delete configured(\"accounts\") | filter(a => a.id == 1) | size"),
            "is not a clause",
        ),
        (
            "insert configured(\"ledger\") {a: 1}".to_owned(),
            "rows of a declared record",
        ),
        (
            format!("{ACCOUNT}insert configured(\"accounts\") 1"),
            "rows of a declared record",
        ),
        (
            format!("{ACCOUNT}insert configured(\"accounts\")"),
            "names the rows it adds",
        ),
        (
            format!("{ACCOUNT}insert configured(\"accounts\") [] | size"),
            "takes no pipeline",
        ),
        (
            format!("{ACCOUNT}update configured(\"counts\") | set(a => {{count: 0}})"),
            "is `Updatable`, and File is not one",
        ),
        (
            format!("{ACCOUNT}delete configured(\"events\")"),
            "no type until its module",
        ),
        (
            format!("{ACCOUNT}rows = [1, 2]\ndelete rows"),
            "`Deletable`",
        ),
        (
            "insert configured(\"accounts\") {id: 1, owner: \"a\", balance: 1}".to_owned(),
            "which is not a type",
        ),
    ] {
        let error = refusal(&program, &vault);
        assert!(error.contains(says), "{program}\n{error}");
    }
}

#[test]
fn the_three_words_are_recognized_by_position_and_a_mutation_is_a_statement() {
    let (_dir, vault) = configured();
    ledger(&vault);
    // A binding or a call of the same name keeps its meaning.
    assert_eq!(value("update = 1\nupdate + 1", &vault), "2");
    assert_eq!(
        value("fn insert(x: Int) : Int = x + 1\ninsert(1)", &vault),
        "2"
    );
    assert_eq!(value("delete = [1]\ndelete | size", &vault), "1");
    // A mutation is the whole of a statement or of a binding's value, and
    // its value is an `Int`.
    assert_eq!(
        checked(
            &format!("{ACCOUNT}n = delete configured(\"accounts\") | filter(a => a.id == 1)\nn"),
            &vault
        ),
        "Int"
    );
    // A table that does not exist is not created by a mutation.
    let error = refusal(
        &format!("{ACCOUNT}insert configured(\"ledger\") Account(id = 1, owner = \"a\", balance = 1)"),
        &vault,
    );
    assert!(error.contains("does not exist"), "{error}");
}

#[test]
fn a_mutation_over_postgres_checks_offline_as_one_statement() {
    let (_dir, vault) = configured();
    let order = "type Order { id : Int, customer : String, total : Decimal }\n";
    assert_eq!(
        checked(
            &format!(
                "import io.db.postgres\n{order}\
                 t = postgres(dsn_env = \"D\", table = \"orders\")\n\
                 update t | of_type(Order) | filter(o => o.id == 1) | set(o => {{total: o.total - 1.50}})"
            ),
            &vault
        ),
        "Int"
    );
    assert_eq!(
        checked(
            &format!("{order}insert configured(\"orders\") Order(id = 1, customer = \"a\", total = 1.00)"),
            &vault
        ),
        "Int"
    );
    assert_eq!(
        checked(r#"delete configured("orders") | filter(o => o.id == 1)"#, &vault),
        "Int"
    );
    // The connection string is read where the statement runs.
    let error = refusal(r#"delete configured("orders")"#, &vault);
    assert!(error.contains("HQL_TEST_UNSET_DSN"), "{error}");
}

#[test]
fn one_gives_the_element_of_a_sequence_of_exactly_one() {
    let (_dir, vault) = configured();
    assert_eq!(value("[7] | one", &vault), "7");
    assert_eq!(value("[1, 2, 3] | filter(x => x > 2) | one", &vault), "3");
    assert_eq!(checked("[\"a\"] | one", &vault), "String");
    let error = refusal("[1, 2] | one", &vault);
    assert!(error.contains("holds 2"), "{error}");
    let error = refusal("[1] | filter(x => x > 5) | one", &vault);
    assert!(error.contains("holds 0"), "{error}");
}

// Phase 3: the transaction scope over SQLite, run, with its handle's
// control calls, its lifecycle and its refusals.

const TRANSFER: &str = "type Transfer { id : Int, from_id : Int, to_id : Int, amount : Int }\n";

/// The ledger: accounts and an empty transfers table, through the
/// configured endpoints.
fn seeded_ledger(vault: &Vault) {
    ledger(vault);
    value(
        &format!(
            "{TRANSFER}[Transfer(id = 0, from_id = 0, to_id = 0, amount = 0)] | target(\"transfers\")\n\
             delete configured(\"transfers\")"
        ),
        vault,
    );
}

const TRANSFER_HEAD: &str = "\
accounts = configured(\"accounts\")
transfers = configured(\"transfers\")
ledger = {accounts, transfers}
";

const TRANSFER_SCOPE: &str = "\
with transaction(attempts = 3) tx on ledger {
    sender = source(accounts) | filter(a => a.id == 1) | one
    tx.rollback(\"insufficient balance\") given sender.balance < 100
    update accounts | filter(a => a.id == 1) | set(a => {balance: a.balance - 100})
    update accounts | filter(a => a.id == 2) | set(a => {balance: a.balance + 100})
    after = source(accounts) | filter(a => a.id == 1) | one
    insert transfers Transfer(id = after.balance, from_id = 1, to_id = 2, amount = 100)
}";

#[test]
fn a_scope_commits_its_statements_together_and_reads_its_own_writes() {
    let (_dir, vault) = configured();
    seeded_ledger(&vault);
    let program = format!("{ACCOUNT}{TRANSFER}{TRANSFER_HEAD}{TRANSFER_SCOPE}");
    assert_eq!(
        checked(&program, &vault),
        "Result<Int, TransactionFailure>"
    );
    // Two transfers commit; the `insert` reads the balance the same attempt
    // updated, so its id is the sender's new balance.
    assert_eq!(value(&program, &vault), "ok(1)");
    assert_eq!(value(&program, &vault), "ok(1)");
    assert_eq!(balances(&vault), "[[1, 50], [2, 240], [3, 9]]");
    assert_eq!(
        value(&format!("{TRANSFER}source(\"transfers\") | map(t => t.id)"), &vault),
        "[50, 150]"
    );
    // The third rolls back by its own rule, and nothing of it remains.
    assert_eq!(
        value(&program, &vault),
        r#"err(TransactionFailure(kind = "rolled_back", reason = "insufficient balance", attempts = 1, endpoint = none, detail = none))"#
    );
    assert_eq!(balances(&vault), "[[1, 50], [2, 240], [3, 9]]");
    // The failure is a value the program handles.
    let handled = value(
        &format!(
            "{ACCOUNT}{TRANSFER}{TRANSFER_HEAD}outcome = {TRANSFER_SCOPE}\n\
             match outcome {{ ok(n) => \"moved\", err(f) => f.kind + \": \" + f.reason }}"
        ),
        &vault,
    );
    assert_eq!(handled, "rolled_back: insufficient balance");
}

#[test]
fn the_handle_ends_an_attempt_early_and_the_block_binds_its_own_names() {
    let (_dir, vault) = configured();
    seeded_ledger(&vault);
    let program = format!(
        "{ACCOUNT}accounts = configured(\"accounts\")\n\
         a = with transaction tx on accounts {{ match source(accounts) | filter(a => a.id == 9) | first {{ some(x) => x.id, none => tx.rollback(\"no such account\") }} }}\n\
         b = with transaction(attempts = 2) tx on accounts {{ tx.retry() }}\n\
         c = with transaction tx on accounts {{ tx.commit(42) given true otherwise 0 }}\n\
         d = with transaction on accounts {{ update accounts | set(a => {{balance: a.balance + 1}}) }}\n\
         e = with transaction tx on accounts {{ n = update accounts | filter(a => a.id == 1) | set(a => {{balance: 0}})\n tx.rollback(\"none\") given n == 0\n n }}\n\
         {{a: a, b: b, c: c, d: d, e: e}}"
    );
    assert_eq!(
        value(&program, &vault),
        r#"{a: "err(TransactionFailure(kind = \"rolled_back\", reason = \"no such account\", attempts = 1, endpoint = none, detail = none))", b: "err(TransactionFailure(kind = \"exhausted\", reason = \"`retry` asked for another attempt after 2\", attempts = 2, endpoint = none, detail = none))", c: "ok(42)", d: "ok(3)", e: "ok(1)"}"#
    );
    assert_eq!(balances(&vault), "[[1, 0], [2, 41], [3, 10]]");
    // A binding of the block is not visible after it.
    let error = refusal(
        &format!("{ACCOUNT}accounts = configured(\"accounts\")\nwith transaction tx on accounts {{ n = 1 }}\nn"),
        &vault,
    );
    assert!(error.contains("`n` is not bound"), "{error}");
}

#[test]
fn a_failure_in_an_attempt_rolls_it_back_and_a_constraint_or_a_conflict_is_data() {
    let (dir, vault) = configured();
    seeded_ledger(&vault);
    let path = dir.path().join("shop.db");
    // A diagnostic inside the block leaves nothing written.
    let error = refusal(
        &format!(
            "{ACCOUNT}accounts = configured(\"accounts\")\n\
             with transaction tx on accounts {{ update accounts | set(a => {{balance: 0}})\n 1 / 0 }}"
        ),
        &vault,
    );
    assert!(error.contains("zero"), "{error}");
    assert_eq!(balances(&vault), "[[1, 250], [2, 40], [3, 9]]");
    // A constraint the system enforces refuses the attempt as data.
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE UNIQUE INDEX owner_once ON accounts(owner)")
        .unwrap();
    let outcome = value(
        &format!(
            "{ACCOUNT}accounts = configured(\"accounts\")\n\
             with transaction tx on accounts {{\n\
                 update accounts | filter(a => a.id == 3) | set(a => {{balance: 1}})\n\
                 insert accounts Account(id = 4, owner = \"ada\", balance = 1)\n\
             }}"
        ),
        &vault,
    );
    assert!(
        outcome.starts_with("err(TransactionFailure(kind = \"constraint\"")
            && outcome.contains("UNIQUE constraint failed"),
        "{outcome}"
    );
    assert_eq!(balances(&vault), "[[1, 250], [2, 40], [3, 9]]");
    // A database another connection holds is a conflict, retried until the
    // attempts are exhausted.
    let holder = rusqlite::Connection::open(&path).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let outcome = value(
        &format!(
            "{ACCOUNT}accounts = configured(\"accounts\")\n\
             with transaction(attempts = 3) tx on accounts {{ update accounts | set(a => {{balance: 0}}) }}"
        ),
        &vault,
    );
    assert!(
        outcome.starts_with("err(TransactionFailure(kind = \"conflict\"")
            && outcome.contains("attempts = 3"),
        "{outcome}"
    );
    // Released, the next scope commits.
    holder.execute_batch("ROLLBACK").unwrap();
    drop(holder);
    assert_eq!(
        value(
            &format!(
                "{ACCOUNT}accounts = configured(\"accounts\")\n\
                 with transaction(attempts = 3) tx on accounts {{ update accounts | set(a => {{balance: 0}}) }}"
            ),
            &vault,
        ),
        "ok(3)"
    );
}

#[test]
fn a_scope_is_checked_for_its_participants_its_handle_and_its_arguments() {
    let (_dir, vault) = configured();
    seeded_ledger(&vault);
    let head = format!("{ACCOUNT}{TRANSFER}accounts = configured(\"accounts\")\ntransfers = configured(\"transfers\")\n");
    for (program, says) in [
        (
            "with transaction tx on accounts { with transaction u on accounts { 1 } }".to_owned(),
            "not written inside another",
        ),
        (
            "with transaction tx on accounts { update transfers | set(t => {amount: 0}) }".to_owned(),
            "no participant of this scope",
        ),
        (
            "with transaction tx on accounts { source(configured(\"transfers\")) | size }".to_owned(),
            "no participant of this scope",
        ),
        (
            "with transaction tx on accounts { [1] | map(x => tx.rollback(\"x\")) }".to_owned(),
            "a control call is a statement of the block",
        ),
        (
            "fn f(t: Transaction) : Int = 1\nwith transaction tx on accounts { f(tx) }".to_owned(),
            "does not leave it",
        ),
        (
            "with transaction tx on accounts { tx.commit(1)\n 2 }".to_owned(),
            "never runs",
        ),
        (
            "with transaction tx on {accounts, configured(\"elsewhere\")} { 1 }".to_owned(),
            "in one database file, and these are in two",
        ),
        (
            "with transaction tx on {accounts, configured(\"events\")} { 1 }".to_owned(),
            "no type until its module",
        ),
        (
            "with transaction tx on configured(\"counts\") { 1 }".to_owned(),
            "runs over a `Transactional` endpoint",
        ),
        (
            "ps = [accounts] | Set\nwith transaction tx on ps { 1 }".to_owned(),
            "known when the program is checked",
        ),
        (
            "with transaction(isolation = RepeatableRead) tx on accounts { 1 }".to_owned(),
            "offers `Serializable` only",
        ),
        (
            "with transaction(isolation = Loose) tx on accounts { 1 }".to_owned(),
            "is not an isolation level",
        ),
        (
            "with transaction(attempts = \"3\") tx on accounts { 1 }".to_owned(),
            "`attempts` is an Int",
        ),
        (
            "with transaction(attempts = 1, attempts = 2) tx on accounts { 1 }".to_owned(),
            "written once",
        ),
        (
            "with transaction tx on accounts { }".to_owned(),
            "at least one statement",
        ),
        (
            "with transaction tx on accounts { tx.commit(1) given true otherwise {2} }".to_owned(),
            "no common type",
        ),
    ] {
        let program = format!("{head}{program}");
        let error = refusal(&program, &vault);
        assert!(error.contains(says), "{program}\n{error}");
    }
    // A name beside the words keeps its meaning.
    assert_eq!(value("with = 1\ntransaction = 2\nwith + transaction", &vault), "3");
    // Isolation is accepted where the kind offers it, and the handle may be
    // left out.
    assert_eq!(
        checked(
            &format!("{head}with transaction(isolation = Serializable) on {{accounts, transfers}} {{ 1 }}"),
            &vault
        ),
        "Result<Int, TransactionFailure>"
    );
    let error = refusal(
        "import io.db.spark\nwith transaction tx on {spark(table = \"a\", url_env = \"L\"), spark(table = \"b\", url_env = \"L\")} { 1 }",
        &vault,
    );
    assert!(error.contains("holds one table"), "{error}");
    assert_eq!(
        checked(
            "import io.db.postgres\nwith transaction(isolation = RepeatableRead, attempts = 2) tx on \
             {postgres(dsn_env = \"D\", table = \"a\"), postgres(dsn_env = \"D\", table = \"b\")} { tx.retry() }",
            &vault
        ),
        "Result<Never, TransactionFailure>"
    );
    let error = refusal(
        "import io.db.postgres\nwith transaction tx on \
         {postgres(dsn_env = \"D\", table = \"a\"), postgres(dsn_env = \"E\", table = \"b\")} { 1 }",
        &vault,
    );
    assert!(error.contains("in one database, and these are in two"), "{error}");
}

// Phase 5: the `spark` kind, checked offline: one table, one statement.

#[test]
fn a_spark_table_is_constructed_by_its_module_and_takes_one_statement_per_scope() {
    let (_dir, vault) = configured();
    let error = refusal(r#"source(spark(table = "t", url_env = "L"))"#, &vault);
    assert!(error.contains("import io.db.spark"), "{error}");
    assert_eq!(
        checked("import io.db.spark\nspark(table = \"t\", url_env = \"L\")", &vault),
        "Spark"
    );
    for capability in ["Network", "RowTransport", "Transactional", "Insertable", "Updatable", "Deletable"] {
        assert_eq!(
            checked(
                &format!("import io.db.spark\nt : {capability} = spark(table = \"t\", url = \"http://h/cliservice\")\nt"),
                &vault
            ),
            capability,
            "{capability}"
        );
    }
    assert_eq!(checked(r#"configured("lake")"#, &vault), "Spark");
    // A mutation checks as one statement; the address is read where it
    // runs.
    assert_eq!(
        checked(
            &format!("{ACCOUNT}update configured(\"lake\") | filter(a => a.id == 1) | set(a => {{balance: a.balance + 1}})"),
            &vault
        ),
        "Int"
    );
    let error = refusal(
        &format!("{ACCOUNT}delete configured(\"lake\") | filter(a => a.id == 1)"),
        &vault,
    );
    assert!(error.contains("HQL_TEST_UNSET_LAKE"), "{error}");
    // A scope over a Delta table holds that table and at most one
    // statement that changes it, at the table's own isolation.
    assert_eq!(
        checked(
            &format!(
                "{ACCOUNT}lake = configured(\"lake\")\n\
                 with transaction(attempts = 2) tx on lake {{\n\
                     n = update lake | filter(a => a.id == 1) | set(a => {{balance: 0}})\n\
                     tx.rollback(\"none\") given n == 0\n n }}"
            ),
            &vault
        ),
        "Result<Int, TransactionFailure>"
    );
    for (program, says) in [
        (
            "with transaction tx on {configured(\"lake\"), configured(\"lake\")} { 1 }",
            "holds one table",
        ),
        (
            "lake = configured(\"lake\")\nwith transaction tx on lake { delete lake\n delete lake }",
            "at most one statement that changes the table",
        ),
        (
            "lake = configured(\"lake\")\nwith transaction(isolation = Serializable) tx on lake { delete lake }",
            "names none",
        ),
        // Two kinds in one set have no common type before the scope's
        // own rule is reached (HQL-0051, "The participants").
        (
            "with transaction tx on {configured(\"lake\"), configured(\"accounts\")} { 1 }",
            "no common type",
        ),
    ] {
        let program = format!("{ACCOUNT}{program}");
        let error = refusal(&program, &vault);
        assert!(error.contains(says), "{program}\n{error}");
    }
}
